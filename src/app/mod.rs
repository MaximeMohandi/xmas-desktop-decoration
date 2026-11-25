//! L'application: état de l'Overlay (simulation dans `sim`, dessin dans `render`).
use crate::prelude::*;

mod render;
mod sim;

pub(crate) struct App {
    pub(crate) hwnd: HWND,
    pub(crate) swap: IDXGISwapChain1,
    pub(crate) ctx: ID2D1DeviceContext,
    pub(crate) brush: ID2D1SolidColorBrush,
    pub(crate) pen: ID2D1SolidColorBrush,
    pub(crate) factory: ID2D1Factory1,
    pub(crate) round: ID2D1StrokeStyle,
    pub(crate) spr: Sprites,
    pub(crate) sparks: Vec<Spark>,
    pub(crate) spark_acc: f32,
    /// Deux groupes sur la barre des tâches: (position relative, pièce haute (sapin?, indice), deux petites pièces).
    pub(crate) salt: u32,
    pub(crate) santa_x: Option<f32>,
    pub(crate) santa_dist: f32,
    pub(crate) santa_pause: f32,
    pub(crate) santa_wave_x: f32,
    pub(crate) santa_waved: bool,
    pub(crate) next_santa: f32,
    pub(crate) sleigh: Option<(f32, f32)>,
    pub(crate) next_sleigh: f32,
    pub(crate) _dcomp: IDCompositionDevice,
    pub(crate) _target: IDCompositionTarget,
    pub(crate) _visual: IDCompositionVisual,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) flakes: Vec<Flake>,
    pub(crate) rng: Rng,
    pub(crate) t: f32,
    pub(crate) active: bool,
    pub(crate) cfg: Settings,
    pub(crate) hidden_for_fullscreen: bool,
    pub(crate) ticks: u32,
    pub(crate) last: Instant,
    pub(crate) last_fs_check: Instant,
    pub(crate) debris: Vec<Debris>,
    pub(crate) anchors: HashMap<isize, Anchor>,
    pub(crate) wins: Vec<WinInfo>,
}

thread_local! { pub(crate) static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

impl App {
    pub(crate) fn new(hwnd: HWND, w: i32, h: i32) -> Result<App> {
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
            let flakes = (0..400)
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
                cfg: Settings::load(),
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

}
