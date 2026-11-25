//! sneakyXmas — prototype: Overlay with snow, tray icon, fullscreen detection.
#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use windows::core::{w, Interface, Result};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const COL_W: f32 = 2.0; // largeur d'une colonne de neige accumulée
const PILE_MAX: f32 = 12.0;
const BUMP_SIGMA: f32 = 5.0; // largeur (en colonnes) de la bosse déposée par un flocon
const BUMP_AMP: f32 = 0.5; // hauteur (px) de cette bosse
const REPOSE: f32 = 1.0; // pente maximale entre colonnes voisines (angle d'éboulement)

const WM_TRAY: u32 = WM_APP + 1;
const ID_TOGGLE: usize = 1001;
const ID_QUIT: usize = 1002;
const ID_SANTA: usize = 1003;
const ID_SLEIGH: usize = 1004;

struct Flake {
    x: f32,
    y: f32,
    r: f32,
    speed: f32,
    phase: f32,
}

/// Fenêtre visible de premier niveau (pour l'ordre d'affichage et le recouvrement).
struct WinInfo {
    hwnd: isize,
    l: f32,
    t: f32,
    r: f32,
    b: f32,
    snowy: bool,
}

/// Ancre: un bord supérieur sur lequel la neige s'accumule (barre des tâches ou fenêtre).
struct Anchor {
    l: f32,
    t: f32,
    r: f32,
    pile: Vec<f32>,
    /// Position dans l'ordre d'affichage (0 = au-dessus de tout). None: jamais recouverte.
    z: Option<usize>,
    /// Position à la fin de l'image précédente (NaN = inconnue), pour détecter un déplacement.
    px: f32,
    py: f32,
    /// Secondes écoulées depuis le dernier déplacement (les décorations réapparaissent ensuite).
    still: f32,
    /// Graine stable pour varier les décorations d'une fenêtre à l'autre.
    seed: u32,
    /// Décoration de la fenêtre, tirée une seule fois à sa création: indice (scènes puis accents), variante,
    /// coin (gauche ou droite) et hauteur. `deco` = visibilité (fondu 0..1).
    kind: usize,
    variant: u32,
    left: bool,
    hh: f32,
    deco: f32,
}

impl Anchor {
    fn set_geometry(&mut self, l: f32, t: f32, r: f32) {
        self.l = l;
        self.t = t;
        self.r = r;
        let n = ((r - l) / COL_W) as usize + 2;
        if self.pile.len() != n {
            self.pile.resize(n, 0.0);
        }
    }
    fn col(&self, x: f32) -> Option<usize> {
        if x >= self.l && x < self.r {
            Some(((x - self.l) / COL_W) as usize).filter(|c| *c < self.pile.len())
        } else {
            None
        }
    }
}

/// Vrai si une fenêtre située au-dessus (indice < z) recouvre le point.
fn covered(wins: &[WinInfo], z: Option<usize>, x: f32, y: f32) -> bool {
    let Some(z) = z else { return false };
    wins.iter().take(z).any(|w| x >= w.l && x < w.r && y >= w.t && y < w.b)
}

thread_local! { static TASKBAR_Z: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
thread_local! { static ENUM_WINS: RefCell<Vec<WinInfo>> = const { RefCell::new(Vec::new()) }; }

fn class_name(hwnd: HWND) -> String {
    let mut cls = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut cls) } as usize;
    String::from_utf16_lossy(&cls[..n])
}

/// Rectangle visible réel (sans les bordures invisibles de Windows 10/11).
fn frame_bounds(hwnd: HWND) -> Option<(f32, f32, f32, f32)> {
    unsafe {
        let mut r = RECT::default();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_ok();
        if !ok && GetWindowRect(hwnd, &mut r).is_err() {
            return None;
        }
        Some((r.left as f32, r.top as f32, r.right as f32, r.bottom as f32))
    }
}

unsafe extern "system" fn enum_cb(hwnd: HWND, _: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return BOOL(1);
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0 {
        return BOOL(1);
    }
    let cls = class_name(hwnd);
    if cls == "Shell_TrayWnd" {
        // Position de la barre des tâches dans l'ordre d'affichage (pour son recouvrement).
        let n = ENUM_WINS.with(|w| w.borrow().len());
        TASKBAR_Z.with(|z| z.set(Some(n)));
        return BOOL(1);
    }
    if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
        return BOOL(1);
    }
    if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_SecondaryTrayWnd" | "SneakyXmasOverlay") {
        return BOOL(1);
    }
    let Some((l, t, r, b)) = frame_bounds(hwnd) else { return BOOL(1) };
    if r - l < 50.0 || b - t < 30.0 {
        return BOOL(1);
    }
    let snowy = !IsZoomed(hwnd).as_bool(); // toutes les fenêtres non maximisées
    ENUM_WINS.with(|w| w.borrow_mut().push(WinInfo { hwnd: hwnd.0 as isize, l, t, r, b, snowy }));
    BOOL(1)
}

/// Paquet de neige détaché d'un tas quand sa fenêtre bouge.
struct Debris {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    a: f32,
}

/// La neige glisse de la fenêtre: le tas devient des paquets qui tombent.
fn shake_off(a: &mut Anchor, debris: &mut Vec<Debris>, rng: &mut Rng) {
    for (i, h) in a.pile.iter_mut().enumerate() {
        if *h > 1.5 && i % 4 == 0 && debris.len() < 600 {
            debris.push(Debris {
                x: a.l + i as f32 * COL_W,
                y: a.t - *h,
                vx: (rng.next() - 0.5) * 120.0,
                vy: rng.next() * 60.0,
                r: 1.5 + rng.next() * 2.5,
                a: 1.0,
            });
        }
        *h = 0.0;
    }
}

struct Sprites {
    deer: [ID2D1Bitmap1; 4],
    sleigh: ID2D1Bitmap1,
    trees: [ID2D1Bitmap1; 3],
    /// Personnages et objets qui se tiennent debout (bonhommes de neige, pingouin, ours, cadeaux...).
    chars: Vec<ID2D1Bitmap1>,
    santa_walk: Vec<ID2D1Bitmap1>,
    santa_wave: ID2D1Bitmap1,
    /// Collection d'icônes (40 éléments, voir `Src::Ico`).
    ico: Vec<ID2D1Bitmap1>,
}

fn load_bitmap(ctx: &ID2D1DeviceContext, wic: &IWICImagingFactory, data: &[u8]) -> Result<ID2D1Bitmap1> {
    unsafe {
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(data)?;
        let dec = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let frame = dec.GetFrame(0)?;
        let conv = wic.CreateFormatConverter()?;
        conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)?;
        ctx.CreateBitmapFromWicBitmap(&conv, None)
    }
}

impl Sprites {
    unsafe fn load(ctx: &ID2D1DeviceContext) -> Result<Sprites> {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let l = |d: &[u8]| load_bitmap(ctx, &wic, d);
        Ok(Sprites {
            deer: [
                l(include_bytes!("../assets/reindeer_1.png"))?,
                l(include_bytes!("../assets/reindeer_2.png"))?,
                l(include_bytes!("../assets/reindeer_3.png"))?,
                l(include_bytes!("../assets/reindeer_4.png"))?,
            ],
            sleigh: l(include_bytes!("../assets/sleigh.png"))?,
            trees: [
                l(include_bytes!("../assets/tree_1.png"))?,
                l(include_bytes!("../assets/tree_2.png"))?,
                l(include_bytes!("../assets/tree_3.png"))?,
            ],
            santa_walk: vec![
                l(include_bytes!("../assets/santa_walk_1.png"))?,
                l(include_bytes!("../assets/santa_walk_2.png"))?,
                l(include_bytes!("../assets/santa_walk_3.png"))?,
                l(include_bytes!("../assets/santa_walk_4.png"))?,
                l(include_bytes!("../assets/santa_walk_5.png"))?,
                l(include_bytes!("../assets/santa_walk_6.png"))?,
                l(include_bytes!("../assets/santa_walk_7.png"))?,
                l(include_bytes!("../assets/santa_walk_8.png"))?,
            ],
            santa_wave: l(include_bytes!("../assets/santa_wave.png"))?,
            ico: vec![
                l(include_bytes!("../assets/ico_01.png"))?,
                l(include_bytes!("../assets/ico_02.png"))?,
                l(include_bytes!("../assets/ico_03.png"))?,
                l(include_bytes!("../assets/ico_04.png"))?,
                l(include_bytes!("../assets/ico_05.png"))?,
                l(include_bytes!("../assets/ico_06.png"))?,
                l(include_bytes!("../assets/ico_07.png"))?,
                l(include_bytes!("../assets/ico_08.png"))?,
                l(include_bytes!("../assets/ico_09.png"))?,
                l(include_bytes!("../assets/ico_10.png"))?,
                l(include_bytes!("../assets/ico_11.png"))?,
                l(include_bytes!("../assets/ico_12.png"))?,
                l(include_bytes!("../assets/ico_13.png"))?,
                l(include_bytes!("../assets/ico_14.png"))?,
                l(include_bytes!("../assets/ico_15.png"))?,
                l(include_bytes!("../assets/ico_16.png"))?,
                l(include_bytes!("../assets/ico_17.png"))?,
                l(include_bytes!("../assets/ico_18.png"))?,
                l(include_bytes!("../assets/ico_19.png"))?,
                l(include_bytes!("../assets/ico_20.png"))?,
                l(include_bytes!("../assets/ico_21.png"))?,
                l(include_bytes!("../assets/ico_22.png"))?,
                l(include_bytes!("../assets/ico_23.png"))?,
                l(include_bytes!("../assets/ico_24.png"))?,
                l(include_bytes!("../assets/ico_25.png"))?,
                l(include_bytes!("../assets/ico_26.png"))?,
                l(include_bytes!("../assets/ico_27.png"))?,
                l(include_bytes!("../assets/ico_28.png"))?,
                l(include_bytes!("../assets/ico_29.png"))?,
                l(include_bytes!("../assets/ico_30.png"))?,
                l(include_bytes!("../assets/ico_31.png"))?,
                l(include_bytes!("../assets/ico_32.png"))?,
                l(include_bytes!("../assets/ico_33.png"))?,
                l(include_bytes!("../assets/ico_34.png"))?,
                l(include_bytes!("../assets/ico_35.png"))?,
                l(include_bytes!("../assets/ico_36.png"))?,
                l(include_bytes!("../assets/ico_37.png"))?,
                l(include_bytes!("../assets/ico_38.png"))?,
                l(include_bytes!("../assets/ico_39.png"))?,
                l(include_bytes!("../assets/ico_40.png"))?,
            ],
            chars: vec![
                l(include_bytes!("../assets/ch_p1.png"))?,
                l(include_bytes!("../assets/ch_p2.png"))?,
                l(include_bytes!("../assets/ch_p3.png"))?,
                l(include_bytes!("../assets/ch_p4.png"))?,
                l(include_bytes!("../assets/ch_p5.png"))?,
                l(include_bytes!("../assets/ch_p6.png"))?,
                l(include_bytes!("../assets/ch_s1.png"))?,
                l(include_bytes!("../assets/ch_s2.png"))?,
                l(include_bytes!("../assets/ch_s3.png"))?,
                l(include_bytes!("../assets/ch_s4.png"))?,
                l(include_bytes!("../assets/ch_s5.png"))?,
                l(include_bytes!("../assets/ch_s6.png"))?,
                l(include_bytes!("../assets/wr_2.png"))?,
                l(include_bytes!("../assets/ic_3.png"))?,
            ],
        })
    }
}

/// Étincelle de la traînée du traîneau.
struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
    life: f32,
}

const SLEIGH_K: f32 = 0.42; // échelle du traîneau (pixels d'image -> pixels d'écran)
/// Source d'une image de décor: collection d'icônes (numérotée comme dans la planche, de 1 à 40),
/// personnages/objets debout, sapins, ou un bonhomme de neige au hasard (selon la variante).
#[derive(Clone, Copy)]
enum Src {
    Ico(usize),
    Chr(usize),
    Tree(usize),
    Snowman,
}
use Src::*;

/// Une pièce d'une scène: hauteur (fraction de la hauteur de scène) et décalage horizontal (en hauteurs de scène).
#[derive(Clone, Copy)]
struct Piece {
    src: Src,
    h: f32,
    dx: f32,
}
const fn pc(src: Src, h: f32, dx: f32) -> Piece {
    Piece { src, h, dx }
}

/// Compositions pensées: la pièce principale d'abord (dessinée devant), puis les accessoires de chaque côté.
static SCENES: [&[Piece]; 8] = [
    // Bonhomme de neige avec un cadeau et une lanterne
    &[pc(Snowman, 1.0, 0.0), pc(Ico(4), 0.30, -0.52), pc(Ico(37), 0.55, 0.50)],
    // Pile de cadeaux, canne en sucre
    &[pc(Chr(12), 0.62, 0.0), pc(Ico(18), 0.55, -0.42), pc(Ico(6), 0.28, 0.42)],
    // Petite forêt: sapin enneigé, sapin simple, jeune renne
    &[pc(Tree(0), 1.0, 0.0), pc(Ico(3), 0.70, -0.55), pc(Chr(5), 0.60, 0.55)],
    // Pôle: ours, pingouin, enseigne
    &[pc(Chr(4), 0.80, 0.0), pc(Chr(3), 0.62, -0.55), pc(Ico(36), 0.55, 0.50)],
    // Maison en pain d'épices, bougie, biscuit
    &[pc(Ico(20), 0.70, 0.0), pc(Ico(21), 0.40, -0.50), pc(Ico(19), 0.45, 0.48)],
    // Cheminée, cadeau, lanterne
    &[pc(Ico(39), 0.65, 0.0), pc(Ico(4), 0.30, -0.45), pc(Ico(37), 0.55, 0.50)],
    // Sapin décoré, clôture, cadeau vert
    &[pc(Tree(1), 1.0, 0.0), pc(Ico(38), 0.40, 0.55), pc(Ico(5), 0.30, -0.50)],
    // Bonhomme à bonnet rouge, pingouin, cadeau
    &[pc(Chr(1), 1.0, 0.0), pc(Chr(3), 0.60, -0.55), pc(Ico(5), 0.28, 0.50)],
];

/// Scènes fixes de la barre des tâches (fournies une fois pour toutes, sans tirage au hasard).
static TB_LEFT: [Piece; 3] = [pc(Tree(1), 1.0, 0.0), pc(Ico(37), 0.55, -0.52), pc(Ico(20), 0.55, 0.55)]; // sapin, lanterne, maison
static TB_RIGHT: [Piece; 2] = [pc(Chr(0), 1.0, 0.0), pc(Ico(4), 0.30, -0.50)]; // bonhomme de neige, cadeau

/// Pièces isolées (un seul accent) pour les fenêtres: un seul accent, jamais une scène complète.
static ACCENTS: [(Src, f32); 16] = [
    (Ico(18), 0.60), // canne en sucre
    (Ico(30), 0.55), // cloche
    (Ico(10), 0.40), // poinsettia
    (Ico(16), 0.50), // étoile
    (Ico(28), 0.45), // moufle
    (Ico(27), 0.60), // chaussette
    (Ico(21), 0.50), // bougie
    (Ico(37), 0.70), // lanterne
    (Ico(36), 0.70), // enseigne
    (Ico(19), 0.60), // pain d'épices
    (Ico(11), 0.40), // houx
    (Ico(8), 0.42),  // boule dorée
    (Chr(3), 0.80),  // pingouin
    (Chr(5), 0.80),  // renne
    (Ico(4), 0.50),  // cadeau
    (Ico(20), 0.80), // maison en pain d'épices
];

/// Fine couche de neige de départ (≈ 1 à 4 px), ondulée de façon douce et propre à chaque graine.
fn floor_h(seed: u32, i: usize) -> f32 {
    let s1 = (seed & 0xFF) as f32 * 0.1;
    let s2 = ((seed >> 8) & 0xFF) as f32 * 0.1;
    let x = i as f32;
    (2.4 + 1.1 * (x * 0.045 + s1).sin() + 0.7 * (x * 0.13 + s2).sin()).max(1.2)
}

const SANTA_K: f32 = 0.40; // père Noël (images de marche)
const SANTA_STRIDE: f32 = 80.0; // distance parcourue pendant un cycle complet de 8 images
const SANTA_SPEED: f32 = 120.0;
const DEER_DS: f32 = 0.55; // taille des rennes par rapport au traîneau

fn col(r: u8, g: u8, b: u8) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

struct App {
    hwnd: HWND,
    swap: IDXGISwapChain1,
    ctx: ID2D1DeviceContext,
    brush: ID2D1SolidColorBrush,
    pen: ID2D1SolidColorBrush,
    factory: ID2D1Factory1,
    round: ID2D1StrokeStyle,
    spr: Sprites,
    sparks: Vec<Spark>,
    spark_acc: f32,
    /// Deux groupes sur la barre des tâches: (position relative, pièce haute (sapin?, indice), deux petites pièces).
    salt: u32,
    santa_x: Option<f32>,
    santa_dist: f32,
    santa_pause: f32,
    santa_wave_x: f32,
    santa_waved: bool,
    next_santa: f32,
    sleigh: Option<(f32, f32)>,
    next_sleigh: f32,
    _dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    w: f32,
    h: f32,
    flakes: Vec<Flake>,
    rng: Rng,
    t: f32,
    active: bool,
    hidden_for_fullscreen: bool,
    ticks: u32,
    last: Instant,
    last_fs_check: Instant,
    debris: Vec<Debris>,
    anchors: HashMap<isize, Anchor>,
    wins: Vec<WinInfo>,
}

thread_local! { static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

impl App {
    fn new(hwnd: HWND, w: i32, h: i32) -> Result<App> {
        unsafe {
            let mut dev: Option<ID3D11Device> = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut dev),
                None,
                None,
            )?;
            let dxgi_dev: IDXGIDevice = dev.unwrap().cast()?;
            // Une seule image d'avance: la position lue est affichée à l'image suivante.
            dxgi_dev.cast::<IDXGIDevice1>()?.SetMaximumFrameLatency(1)?;
            let factory: IDXGIFactory2 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;

            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: w as u32,
                Height: h as u32,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                Stereo: false.into(),
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                Flags: 0,
            };
            let swap = factory.CreateSwapChainForComposition(&dxgi_dev, &desc, None)?;

            let d2d_factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_dev = d2d_factory.CreateDevice(&dxgi_dev)?;
            let ctx = d2d_dev.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let brush = ctx.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.9 },
                None,
            )?;

            let pen = ctx.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
            let spr = Sprites::load(&ctx)?;
            let round = d2d_factory.cast::<ID2D1Factory>()?.CreateStrokeStyle(
                &D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    dashCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    miterLimit: 1.0,
                    dashStyle: D2D1_DASH_STYLE_SOLID,
                    dashOffset: 0.0,
                },
                None,
            )?;

            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi_dev)?;
            let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
            let visual = dcomp.CreateVisual()?;
            visual.SetContent(&swap)?;
            target.SetRoot(&visual)?;
            dcomp.Commit()?;

            let mut rng = Rng(0x2545F4914F6CDD1D);
            let flakes = (0..220)
                .map(|_| Flake {
                    x: rng.next() * w as f32,
                    y: rng.next() * h as f32,
                    r: 1.5 + rng.next() * 3.5,
                    speed: 30.0 + rng.next() * 60.0,
                    phase: rng.next() * 6.28,
                })
                .collect();

            let salt = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1;
            Ok(App {
                hwnd,
                swap,
                ctx,
                brush,
                pen,
                factory: d2d_factory,
                round,
                spr,
                sparks: Vec::new(),
                spark_acc: 0.0,
                salt,
                santa_x: None,
                santa_dist: 0.0,
                santa_pause: 0.0,
                santa_wave_x: 0.0,
                santa_waved: false,
                next_santa: 12.0,
                sleigh: None,
                next_sleigh: 40.0,
                _dcomp: dcomp,
                _target: target,
                _visual: visual,
                w: w as f32,
                h: h as f32,
                flakes,
                rng,
                t: 0.0,
                active: true,
                hidden_for_fullscreen: false,
                ticks: 0,
                last: Instant::now(),
                last_fs_check: Instant::now() - std::time::Duration::from_secs(1),
                debris: Vec::new(),
                anchors: HashMap::new(),
                wins: Vec::new(),
            })
        }
    }

    fn is_rendering(&self) -> bool {
        self.active && !self.hidden_for_fullscreen
    }

    /// Clé 0 = barre des tâches (en bas de l'écran principal uniquement pour l'instant).
    fn update_taskbar(&mut self) {
        unsafe {
            let mut r = RECT::default();
            if let Ok(tb) = FindWindowW(w!("Shell_TrayWnd"), None) {
                if GetWindowRect(tb, &mut r).is_ok()
                    && (r.top as f32) < self.h - 2.0
                    && r.top as f32 > self.h * 0.5
                    && r.left < r.right
                {
                    let a = self.anchors.entry(0).or_insert(Anchor { l: 0.0, t: 0.0, r: 0.0, pile: Vec::new(), z: None, px: f32::NAN, py: f32::NAN, still: 0.0, seed: 0, kind: 0, variant: 0, left: false, hh: 60.0, deco: 0.0 });
                    a.set_geometry(r.left as f32, r.top as f32, r.right as f32);
                    a.still = 99.0; // la barre ne bouge pas: couche de neige dès le départ
                    return;
                }
            }
            self.anchors.remove(&0); // masquée ou ailleurs: pas d'accumulation
        }
    }

    /// Réénumère les fenêtres (ordre d'affichage) et crée/supprime les Ancres.
    fn refresh_windows(&mut self) {
        ENUM_WINS.with(|w| w.borrow_mut().clear());
        TASKBAR_Z.with(|z| z.set(None));
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM(0));
        }
        self.wins = ENUM_WINS.with(|w| std::mem::take(&mut *w.borrow_mut()));
        let mut keep = vec![0isize];
        for (z, w) in self.wins.iter().enumerate() {
            // Place pour le tas au-dessus de la fenêtre.
            if w.snowy && w.t > PILE_MAX + 4.0 && w.t < self.h {
                keep.push(w.hwnd);
                // Décoration tirée une seule fois, à la création de la fenêtre, en évitant celles déjà utilisées.
                let fresh = !self.anchors.contains_key(&w.hwnd);
                let pick = if fresh {
                    let n = SCENES.len() + ACCENTS.len();
                    let used: Vec<usize> = self.anchors.iter().filter(|(k, _)| **k != 0).map(|(_, a)| a.kind).collect();
                    let free: Vec<usize> = (0..n).filter(|k| !used.contains(k)).collect();
                    let kind = if free.is_empty() {
                        (self.rng.next() * n as f32) as usize % n
                    } else {
                        free[(self.rng.next() * free.len() as f32) as usize % free.len()]
                    };
                    (kind, (self.rng.next() * 1000.0) as u32, self.rng.next() < 0.5, 56.0 + self.rng.next() * 16.0)
                } else {
                    (0, 0, false, 0.0)
                };
                let a = self.anchors.entry(w.hwnd).or_insert(Anchor { l: 0.0, t: 0.0, r: 0.0, pile: Vec::new(), z: None, px: f32::NAN, py: f32::NAN, still: 0.0, seed: 0, kind: 0, variant: 0, left: false, hh: 60.0, deco: 0.0 });
                a.set_geometry(w.l, w.t, w.r);
                a.z = Some(z);
                if a.seed == 0 {
                    a.seed = ((w.hwnd as u32).wrapping_mul(2654435761) ^ self.salt) | 1;
                    (a.kind, a.variant, a.left, a.hh) = pick;
                }
            }
        }
        self.anchors.retain(|k, _| keep.contains(k));
        let tz = TASKBAR_Z.with(|z| z.get());
        if let Some(a) = self.anchors.get_mut(&0) {
            a.z = tz;
        }
    }

    /// Suit les fenêtres suivies; si l'une bouge, sa neige se détache.
    fn track_windows(&mut self, dt: f32) {
        for w in self.wins.iter_mut().filter(|w| w.snowy) {
            let Some((l, t, r, b)) = frame_bounds(HWND(w.hwnd as *mut _)) else { continue };
            (w.l, w.t, w.r, w.b) = (l, t, r, b);
            if let Some(a) = self.anchors.get_mut(&w.hwnd) {
                a.set_geometry(l, t, r);
                if !a.px.is_nan() && (l - a.px).abs() + (t - a.py).abs() > 1.0 {
                    shake_off(a, &mut self.debris, &mut self.rng);
                    a.still = 0.0;
                } else {
                    a.still += dt;
                }
                (a.px, a.py) = (l, t);
            }
        }
    }

    fn tick(&mut self) {
        self.ticks += 1;
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
        self.last = now;

        // Mode plein écran / écran verrouillé: on masque et on ne dessine plus.
        if now.duration_since(self.last_fs_check).as_secs_f32() >= 0.5 {
            self.last_fs_check = now;
            let fullscreen = unsafe {
                matches!(
                    SHQueryUserNotificationState(),
                    Ok(QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
                ) || foreground_covers_monitor(self.hwnd)
            };
            if fullscreen != self.hidden_for_fullscreen {
                self.hidden_for_fullscreen = fullscreen;
                self.apply_visibility();
            }
        }
        if !self.is_rendering() {
            return;
        }
        if self.ticks % 30 == 0 {
            self.update_taskbar();
        }
        if self.ticks % 3 == 0 || self.wins.is_empty() {
            self.refresh_windows();
        }
        self.track_windows(dt);
        // Les décorations d'une fenêtre apparaissent en fondu quand elle est immobile.
        for a in self.anchors.values_mut() {
            let target = if a.still > 0.25 { 1.0 } else { 0.0 };
            let step = 3.0 * dt;
            a.deco += (target - a.deco).clamp(-step, step);
        }

        self.t += dt;
        self.update_characters(dt);
        for f in &mut self.flakes {
            let prev_y = f.y;
            f.y += f.speed * dt;
            f.x += (self.t * 0.8 + f.phase).sin() * 20.0 * dt;
            let mut landed = false;
            for (key, a) in self.anchors.iter_mut() {
                let Some(c) = a.col(f.x) else { continue };
                let level = a.t - a.pile[c];
                let is_taskbar = *key == 0;
                // Fenêtre: le flocon doit arriver d'au-dessus du bord, loin des coins.
                // Barre des tâches: tout flocon qui atteint le tas se pose (sinon il traverserait la barre).
                let from_above = prev_y + f.r <= level + 1.0;
                let reaches = f.y + f.r >= level && (from_above || (is_taskbar && f.y < a.t + 60.0));
                let away_from_corner = is_taskbar || (f.x - a.l > 8.0 && a.r - f.x > 8.0);
                if reaches && away_from_corner && !covered(&self.wins, a.z, f.x, a.t - 1.0) {
                    // Une bosse arrondie (gaussienne) plutôt qu'une colonne: le tas grossit en douceur.
                    let rad = (BUMP_SIGMA * 2.6) as i32;
                    for d in -rad..=rad {
                        let idx = c as i32 + d;
                        if idx >= 0 && (idx as usize) < a.pile.len() {
                            let wgt = (-((d * d) as f32) / (2.0 * BUMP_SIGMA * BUMP_SIGMA)).exp();
                            a.pile[idx as usize] = (a.pile[idx as usize] + BUMP_AMP * wgt).min(PILE_MAX);
                        }
                    }
                    landed = true;
                    break;
                }
            }
            if landed || f.y > self.h + 5.0 {
                f.y = -5.0;
                f.x = self.rng.next() * self.w;
            }
        }
        // Angle d'éboulement: la neige ne tient pas en pente raide, elle glisse vers le côté plus bas.
        for a in self.anchors.values_mut() {
            let n = a.pile.len();
            for _ in 0..2 {
                for i in 0..n.saturating_sub(1) {
                    let d = a.pile[i] - a.pile[i + 1];
                    if d > REPOSE {
                        let m = (d - REPOSE) * 0.5;
                        a.pile[i] -= m;
                        a.pile[i + 1] += m;
                    } else if d < -REPOSE {
                        let m = (-d - REPOSE) * 0.5;
                        a.pile[i] += m;
                        a.pile[i + 1] -= m;
                    }
                }
            }
        }
        // Fine couche de neige de départ: elle revient peu après qu'une fenêtre se soit arrêtée.
        for a in self.anchors.values_mut() {
            let scale = ((a.still - 0.3) / 1.5).clamp(0.0, 1.0);
            let sd = a.seed | 1;
            for (i, h) in a.pile.iter_mut().enumerate() {
                let f = floor_h(sd, i) * scale;
                if *h < f {
                    *h = f;
                }
            }
        }
        // Chute des paquets de neige détachés.
        for d in &mut self.debris {
            d.vy += 900.0 * dt;
            d.x += d.vx * dt;
            d.y += d.vy * dt;
            d.a -= 1.1 * dt;
        }
        let h = self.h;
        self.debris.retain(|d| d.a > 0.0 && d.y < h);
        let _ = self.draw();
    }

    /// Père Noël qui marche sur la barre des tâches, traîneau qui traverse le haut de l'écran.
    fn update_characters(&mut self, dt: f32) {
        let tb = self.anchors.get(&0).map(|a| (a.l, a.r));
        if let Some((al, ar)) = tb {
            match self.santa_x {
                Some(x) => {
                    if self.santa_pause > 0.0 {
                        self.santa_pause -= dt;
                    } else {
                        let nx = x + SANTA_SPEED * dt;
                        self.santa_dist += SANTA_SPEED * dt;
                        if nx > ar + 60.0 {
                            self.santa_x = None;
                            self.next_santa = self.t + 120.0 + self.rng.next() * 60.0;
                        } else {
                            self.santa_x = Some(nx);
                            // À mi-parcours il s'arrête pour saluer.
                            if !self.santa_waved && nx >= self.santa_wave_x {
                                self.santa_waved = true;
                                self.santa_pause = 2.6;
                            }
                        }
                    }
                }
                None if self.t >= self.next_santa => {
                    self.santa_x = Some(al - 60.0);
                    self.santa_dist = 0.0;
                    self.santa_pause = 0.0;
                    self.santa_waved = false;
                    self.santa_wave_x = al + (ar - al) * (0.35 + self.rng.next() * 0.3);
                }
                None => {}
            }
        }
        match self.sleigh {
            Some((x, dir)) => {
                let nx = x + dir * (self.w + 1000.0) / 11.0 * dt;
                if nx < -520.0 || nx > self.w + 520.0 {
                    self.sleigh = None;
                    self.next_sleigh = self.t + 240.0 + self.rng.next() * 120.0;
                } else {
                    self.sleigh = Some((nx, dir));
                }
            }
            None if self.t >= self.next_sleigh => {
                let dir = if self.rng.next() < 0.5 { 1.0 } else { -1.0 };
                self.sleigh = Some((if dir > 0.0 { -500.0 } else { self.w + 500.0 }, dir));
            }
            None => {}
        }
        // Traînée d'étincelles derrière le traîneau.
        if let Some((x, dir)) = self.sleigh {
            let y = self.sleigh_y();
            self.spark_acc += dt;
            while self.spark_acc > 0.02 {
                self.spark_acc -= 0.02;
                if self.sparks.len() < 400 {
                    self.sparks.push(Spark {
                        x: x + (self.rng.next() - 0.5) * 14.0,
                        y: y + 90.0 + self.rng.next() * 70.0,
                        vx: -dir * (30.0 + self.rng.next() * 70.0),
                        vy: (self.rng.next() - 0.3) * 40.0,
                        r: 1.2 + self.rng.next() * 2.6,
                        life: 1.0,
                    });
                }
            }
        } else {
            self.spark_acc = 0.0;
        }
        for sp in &mut self.sparks {
            sp.x += sp.vx * dt;
            sp.y += sp.vy * dt;
            sp.vy += 25.0 * dt;
            sp.life -= 0.9 * dt;
        }
        self.sparks.retain(|sp| sp.life > 0.0);
    }

    fn sleigh_y(&self) -> f32 {
        self.h * 0.12 + (self.t * 2.2).sin() * 10.0
    }

    unsafe fn dot(&self, x: f32, y: f32, rx: f32, ry: f32, c: D2D1_COLOR_F) {
        self.pen.SetColor(&c);
        self.ctx.FillEllipse(&D2D1_ELLIPSE { point: D2D_POINT_2F { x, y }, radiusX: rx, radiusY: ry }, &self.pen);
    }
    unsafe fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: D2D1_COLOR_F) {
        self.pen.SetColor(&c);
        self.ctx.DrawLine(D2D_POINT_2F { x: x0, y: y0 }, D2D_POINT_2F { x: x1, y: y1 }, &self.pen, w, &self.round);
    }
    unsafe fn place(&self, x: f32, y: f32, sx: f32, sy: f32) {
        self.ctx.SetTransform(&windows::Foundation::Numerics::Matrix3x2 { M11: sx, M12: 0.0, M21: 0.0, M22: sy, M31: x, M32: y });
    }
    unsafe fn reset(&self) {
        self.place(0.0, 0.0, 1.0, 1.0);
    }

    unsafe fn place_rot(&self, x: f32, y: f32, sx: f32, sy: f32, rot: f32) {
        let (c, s) = (rot.cos(), rot.sin());
        self.ctx.SetTransform(&windows::Foundation::Numerics::Matrix3x2 { M11: sx * c, M12: sx * s, M21: -sy * s, M22: sy * c, M31: x, M32: y });
    }

    /// Dessine une image (coin haut-gauche en (x, y), échelle s) dans l'espace courant.
    unsafe fn spr(&self, b: &ID2D1Bitmap1, x: f32, y: f32, s: f32) {
        self.spr_op(b, x, y, s, 1.0);
    }

    unsafe fn spr_op(&self, b: &ID2D1Bitmap1, x: f32, y: f32, s: f32, op: f32) {
        let sz = b.GetPixelSize();
        let r = D2D_RECT_F { left: x, top: y, right: x + sz.width as f32 * s, bottom: y + sz.height as f32 * s };
        self.ctx.DrawBitmap(b, Some(&r), op, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, None, None);
    }

    unsafe fn bmp(&self, src: Src, variant: u32) -> &ID2D1Bitmap1 {
        match src {
            Ico(i) => &self.spr.ico[(i - 1).min(self.spr.ico.len() - 1)],
            Chr(i) => &self.spr.chars[i % self.spr.chars.len()],
            Tree(i) => &self.spr.trees[i % 3],
            Snowman => &self.spr.chars[[0usize, 1, 2, 6, 7, 8][variant as usize % 6]],
        }
    }

    /// Sprite debout de hauteur `h` px, centre-bas en (x, base_y); se balance et « respire » légèrement.
    unsafe fn draw_h(&self, b: &ID2D1Bitmap1, x: f32, base_y: f32, h: f32, op: f32) {
        let sz = b.GetPixelSize();
        let k = h / sz.height as f32;
        let phase = x * 0.013 + h * 0.05;
        let sway = (self.t * 0.9 + phase).sin() * 0.012;
        let breathe = 1.0 + (self.t * 1.6 + phase).sin() * 0.012;
        self.place_rot(x, base_y, k, k * breathe, sway);
        self.spr_op(b, -(sz.width as f32) / 2.0, -(sz.height as f32), 1.0, op);
        self.reset();
    }

    /// Lueur vacillante (lanterne, bougie).
    unsafe fn glow(&self, x: f32, y: f32, r: f32, op: f32) {
        let flick = 0.5 + 0.5 * (self.t * 7.0 + x).sin() * (self.t * 3.1).sin();
        self.dot(x, y, r * 1.8, r * 1.8, D2D1_COLOR_F { r: 1.0, g: 0.72, b: 0.3, a: (0.05 + 0.04 * flick) * op });
        self.dot(x, y, r, r, D2D1_COLOR_F { r: 1.0, g: 0.8, b: 0.4, a: (0.10 + 0.06 * flick) * op });
    }

    /// Dessine une scène: pièce principale au centre (devant), accessoires de chaque côté, posés sur une congère.
    /// `hh` = hauteur de la scène en px, `mirror` inverse la composition. Renvoie l'étendue horizontale.
    unsafe fn draw_scene(&self, pieces: &[Piece], variant: u32, x: f32, base_y: f32, hh: f32, mirror: bool, op: f32, z: Option<usize>, cover_y: f32) -> (f32, f32) {
        let sgn = if mirror { -1.0 } else { 1.0 };
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut placed: Vec<(Src, f32, f32)> = Vec::new();
        for p in pieces {
            let sz = self.bmp(p.src, variant).GetPixelSize();
            let h = p.h * hh;
            let w = sz.width as f32 * h / sz.height as f32;
            let cx = x + sgn * p.dx * hh;
            lo = lo.min(cx - w / 2.0);
            hi = hi.max(cx + w / 2.0);
            placed.push((p.src, cx, h));
        }
        // Congère sous la scène: un décor posé sur la neige est ancré, pas flottant.
        let mid = (lo + hi) / 2.0;
        if !covered(&self.wins, z, mid, cover_y) {
            let pile = &self.spr.ico[30];
            let psz = pile.GetPixelSize();
            let sx = (hi - lo) * 1.3 / psz.width as f32;
            self.place(mid, base_y + 4.0, sx, sx * 0.55);
            self.spr_op(pile, -(psz.width as f32) / 2.0, -(psz.height as f32), 1.0, op);
            self.reset();
        }
        for (src, cx, h) in placed.iter().rev() {
            if covered(&self.wins, z, *cx, cover_y) {
                continue;
            }
            self.draw_h(self.bmp(*src, variant), *cx, base_y, *h, op);
            match src {
                Ico(37) => self.glow(*cx, base_y - h * 0.60, h * 0.45, op),
                Ico(21) => self.glow(*cx, base_y - h * 0.62, h * 0.40, op),
                _ => {}
            }
        }
        (lo, hi)
    }

    /// Chemin lisse: remplit de la base jusqu'à la courbe passant par les points (courbes quadratiques aux milieux).
    unsafe fn smooth_path(&self, pts: &[(f32, f32)], base: f32) -> Option<ID2D1PathGeometry> {
        if pts.len() < 2 {
            return None;
        }
        let g = self.factory.CreatePathGeometry().ok()?;
        let sink = g.Open().ok()?;
        let p = |x: f32, y: f32| D2D_POINT_2F { x, y };
        sink.BeginFigure(p(pts[0].0, base), D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(p(pts[0].0, pts[0].1));
        for i in 1..pts.len() {
            let mid = ((pts[i - 1].0 + pts[i].0) / 2.0, (pts[i - 1].1 + pts[i].1) / 2.0);
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT { point1: p(pts[i - 1].0, pts[i - 1].1), point2: p(mid.0, mid.1) });
        }
        let last = pts[pts.len() - 1];
        sink.AddLine(p(last.0, last.1));
        sink.AddLine(p(last.0, base));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close().ok()?;
        g.cast().ok()
    }

    /// Tas de neige: forme arrondie qui s'affine aux extrémités, avec une ombre bleutée en bas.
    unsafe fn draw_pile(&self, a: &Anchor) {
        let step = 3usize;
        let base = a.t + 1.0;
        let mut run: Vec<(f32, f32)> = Vec::new(); // (x, hauteur)
        let flush = |run: &mut Vec<(f32, f32)>| {
            if run.len() >= 2 && run.iter().any(|p| p.1 > 0.5) {
                let top: Vec<(f32, f32)> = run.iter().map(|p| (p.0, a.t - p.1)).collect();
                if let Some(g) = self.smooth_path(&top, base) {
                    self.pen.SetColor(&D2D1_COLOR_F { r: 0.97, g: 0.98, b: 1.0, a: 0.98 });
                    self.ctx.FillGeometry(&g, &self.pen, None);
                }
                let shade: Vec<(f32, f32)> = run.iter().map(|p| (p.0, a.t - p.1 * 0.38)).collect();
                if let Some(g) = self.smooth_path(&shade, base) {
                    self.pen.SetColor(&D2D1_COLOR_F { r: 0.78, g: 0.86, b: 0.96, a: 0.9 });
                    self.ctx.FillGeometry(&g, &self.pen, None);
                }
            }
            run.clear();
        };
        let mut i = 0;
        while i < a.pile.len() {
            let x = a.l + i as f32 * COL_W;
            if x > a.r {
                break;
            }
            // Les extrémités s'affinent (coins de fenêtre arrondis).
            let edge = ((x - a.l).min(a.r - x) / 16.0).clamp(0.0, 1.0);
            let taper = edge * edge * (3.0 - 2.0 * edge);
            if covered(&self.wins, a.z, x, a.t - 1.0) {
                flush(&mut run);
            } else {
                run.push((x, a.pile[i] * taper));
            }
            i += step;
        }
        flush(&mut run);
    }

    unsafe fn draw_sleigh(&self, x: f32, dir: f32) {
        let y = self.sleigh_y();
        let k = SLEIGH_K;
        let tilt = (self.t * 2.2 + 0.6).sin() * 0.025;
        let step = (self.t * 9.0) as usize;
        let seq = [0usize, 1, 2, 3, 2, 1];
        let ds = DEER_DS;
        let b1 = (self.t * 9.0).sin() * 3.0;
        let b2 = (self.t * 9.0 + 1.3).sin() * 3.0;
        let (o1, o2) = ((610.0, 100.0 + b1), (840.0, 84.0 + b2));
        let bell = (245.0 * ds, 182.0 * ds); // collier du renne, en pixels du traîneau
        let hand = (516.0, 222.0);

        self.place_rot(x, y, dir * k, k, tilt);
        self.spr(&self.spr.sleigh, 0.0, 0.0, 1.0);
        let rein = |from: (f32, f32), to: (f32, f32)| {
            let c = ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0 + 22.0);
            let mut prev = from;
            for j in 1..=14 {
                let u = j as f32 / 14.0;
                let p = ((1.0 - u) * (1.0 - u) * from.0 + 2.0 * u * (1.0 - u) * c.0 + u * u * to.0,
                         (1.0 - u) * (1.0 - u) * from.1 + 2.0 * u * (1.0 - u) * c.1 + u * u * to.1);
                self.line(prev.0, prev.1, p.0, p.1, 4.5, col(112, 66, 40));
                prev = p;
            }
        };
        rein(hand, (o2.0 + bell.0, o2.1 + bell.1));
        self.spr(&self.spr.deer[seq[(step + 2) % 6]], o2.0, o2.1, ds);
        rein(hand, (o1.0 + bell.0, o1.1 + bell.1));
        self.spr(&self.spr.deer[seq[step % 6]], o1.0, o1.1, ds);
        self.reset();
    }

    unsafe fn draw_santa(&self, x: f32, y: f32) {
        // Ombre douce au sol.
        self.dot(x, y + 1.0, 20.0, 3.5, D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.05, a: 0.18 });
        if self.santa_pause > 0.0 {
            let b = &self.spr.santa_wave;
            let sz = b.GetPixelSize();
            let k = SANTA_K * 0.80;
            self.place_rot(x, y, k, k, (self.t * 3.0).sin() * 0.02);
            self.spr(b, -(sz.width as f32) * 0.5, -(sz.height as f32), 1.0);
        } else {
            let idx = ((self.santa_dist / SANTA_STRIDE) * 8.0) as usize % 8;
            let b = &self.spr.santa_walk[idx];
            let sz = b.GetPixelSize();
            self.place(x, y, SANTA_K, SANTA_K);
            self.spr(b, -(sz.width as f32) * 0.5, -(sz.height as f32), 1.0);
        }
        self.reset();
    }

    fn draw(&self) -> Result<()> {
        unsafe {
            let surface: IDXGISurface = self.swap.GetBuffer(0)?;
            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: std::mem::ManuallyDrop::new(None),
            };
            let bitmap = self.ctx.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
            self.ctx.SetTarget(&bitmap);
            self.ctx.BeginDraw();
            self.ctx.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            if let Some(a) = self.anchors.get(&0) {
                let width = a.r - a.l;
                self.draw_scene(&TB_LEFT, 0, a.l + width * 0.11, a.t + 2.0, 96.0, false, 1.0, a.z, a.t - 10.0);
                self.draw_scene(&TB_RIGHT, 0, a.l + width * 0.82, a.t + 2.0, 74.0, true, 1.0, a.z, a.t - 10.0);
                if let Some(sx) = self.santa_x {
                    if !covered(&self.wins, a.z, sx, a.t - 10.0) {
                        self.draw_santa(sx, a.t - 1.0);
                    }
                }
            }
            // Fenêtres: chacune garde la décoration tirée à sa création (scène ou accent), quand elle est immobile.
            for (key, a) in self.anchors.iter() {
                let width = a.r - a.l;
                if *key == 0 || a.deco < 0.02 || width < 150.0 {
                    continue;
                }
                let hh = a.hh;
                if a.kind < SCENES.len() {
                    // Fenêtre étroite: seule la pièce principale de la scène.
                    let pieces = if width >= 300.0 { SCENES[a.kind] } else { &SCENES[a.kind][..1] };
                    let cx = if a.left { a.l + 16.0 + hh } else { a.r - 16.0 - hh };
                    self.draw_scene(pieces, a.variant, cx, a.t + 1.0, hh, !a.left, a.deco, a.z, a.t - 1.0);
                } else {
                    let (src, rel) = ACCENTS[a.kind - SCENES.len()];
                    let cx = if a.left { a.l + 40.0 } else { a.r - 40.0 };
                    self.draw_scene(&[pc(src, rel, 0.0)], a.variant, cx, a.t + 1.0, hh, false, a.deco, a.z, a.t - 1.0);
                }
            }
            let nw = self.wins.len();
            for f in &self.flakes {
                // Un flocon n'est visible que sur le bureau: jamais devant une fenêtre (ni la barre des tâches).
                if covered(&self.wins, Some(nw), f.x, f.y) {
                    continue;
                }
                let e = D2D1_ELLIPSE {
                    point: D2D_POINT_2F { x: f.x, y: f.y },
                    radiusX: f.r,
                    radiusY: f.r,
                };
                self.ctx.FillEllipse(&e, &self.brush);
            }
            for a in self.anchors.values() {
                self.draw_pile(a);
            }
            for sp in &self.sparks {
                let a = sp.life.clamp(0.0, 1.0);
                self.dot(sp.x, sp.y, sp.r * 2.6, sp.r * 2.6, D2D1_COLOR_F { r: 1.0 * 0.35 * a, g: 0.85 * 0.35 * a, b: 0.5 * 0.35 * a, a: 0.35 * a });
                self.dot(sp.x, sp.y, sp.r, sp.r, D2D1_COLOR_F { r: 1.0 * a, g: 0.93 * a, b: 0.65 * a, a });
            }
            if let Some((sx, dir)) = self.sleigh {
                self.draw_sleigh(sx, dir);
            }
            for d in &self.debris {
                self.brush.SetOpacity(d.a.clamp(0.0, 1.0));
                let e = D2D1_ELLIPSE { point: D2D_POINT_2F { x: d.x, y: d.y }, radiusX: d.r, radiusY: d.r };
                self.ctx.FillEllipse(&e, &self.brush);
            }
            self.brush.SetOpacity(1.0);
            self.ctx.EndDraw(None, None)?;
            self.ctx.SetTarget(None);
            // Libère toute référence au tampon avant Present (sinon scintillement).
            drop(bitmap);
            drop(surface);
            self.swap.Present(2, DXGI_PRESENT(0)).ok()?;
        }
        Ok(())
    }

    fn apply_visibility(&self) {
        unsafe {
            let _ = ShowWindow(
                self.hwnd,
                if self.active && !self.hidden_for_fullscreen { SW_SHOWNOACTIVATE } else { SW_HIDE },
            );
        }
    }
}

/// Vrai si la fenêtre au premier plan (hors Overlay, bureau, barre des tâches)
/// couvre entièrement son écran sans être maximisée: jeu, film, F11 navigateur.
/// Note: SHQueryUserNotificationState ne convient pas seul car il détecte l'Overlay
/// lui-même (il couvre tout l'écran) comme une application plein écran.
fn foreground_covers_monitor(own: HWND) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg == own {
            return false;
        }
        let mut cls = [0u16; 64];
        let n = GetClassNameW(fg, &mut cls) as usize;
        let cls = String::from_utf16_lossy(&cls[..n]);
        if matches!(
            cls.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "SneakyXmasOverlay"
        ) {
            return false;
        }
        // Pas d'exclusion des fenêtres maximisées: F11 / vidéo plein écran le restent pour Windows.
        if !IsWindowVisible(fg).as_bool() {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let mon = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return false;
        }
        let m = mi.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

fn add_tray(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        for (i, c) in "sneakyXmas".encode_utf16().enumerate() {
            nid.szTip[i] = c;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

fn remove_tray(hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn show_menu(hwnd: HWND, active: bool) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let flags = if active { MF_STRING | MF_CHECKED } else { MF_STRING };
        let _ = AppendMenuW(menu, flags, ID_TOGGLE, w!("Mode Noël"));
        let _ = AppendMenuW(menu, MF_STRING, ID_SANTA, w!("Lancer le Père Noël"));
        let _ = AppendMenuW(menu, MF_STRING, ID_SLEIGH, w!("Lancer le traîneau"));
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quitter"));
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, None);
        let _ = DestroyMenu(menu);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            if (lp.0 as u32) == WM_RBUTTONUP || (lp.0 as u32) == WM_CONTEXTMENU {
                let active = APP.with(|a| a.borrow().as_ref().map(|x| x.active).unwrap_or(false));
                show_menu(hwnd, active);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wp.0 & 0xFFFF {
                ID_TOGGLE => APP.with(|a| {
                    if let Some(app) = a.borrow_mut().as_mut() {
                        app.active = !app.active;
                        app.apply_visibility();
                    }
                }),
                ID_SANTA | ID_SLEIGH => {
                    let which = wp.0 & 0xFFFF;
                    APP.with(|a| {
                        if let Some(app) = a.borrow_mut().as_mut() {
                            if which == ID_SANTA {
                                app.next_santa = 0.0;
                            } else {
                                app.next_sleigh = 0.0;
                            }
                        }
                    });
                }
                ID_QUIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn main() {
    if let Err(e) = run() {
        let _ = std::fs::write("sneakyxmas.log", format!("{e:?}\n"));
    }
}

fn run() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hinst = GetModuleHandleW(None)?;
        let class = w!("SneakyXmasOverlay");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Prototype: écran principal uniquement.
        let (w, h) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_TRANSPARENT
                | WS_EX_NOACTIVATE
                | WS_EX_LAYERED
                | WS_EX_NOREDIRECTIONBITMAP,
            class,
            w!("sneakyXmas"),
            WS_POPUP,
            0,
            0,
            w,
            h,
            None,
            None,
            hinst,
            None,
        )?;

        let app = App::new(hwnd, w, h)?;
        APP.with(|a| *a.borrow_mut() = Some(app));
        add_tray(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        let mut msg = MSG::default();
        loop {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            // Present(2) cadence le rendu sur l'écran (30 images/s sur 60 Hz).
            let rendering = APP.with(|a| {
                a.borrow_mut()
                    .as_mut()
                    .map(|app| {
                        app.tick();
                        app.is_rendering()
                    })
                    .unwrap_or(false)
            });
            if !rendering {
                MsgWaitForMultipleObjects(None, false, 250, QS_ALLINPUT);
            }
        }
    }
}

