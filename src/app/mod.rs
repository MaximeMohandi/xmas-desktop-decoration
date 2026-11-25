//! L'application: état de l'Overlay (simulation dans `sim`, dessin dans `render`).
use crate::prelude::*;

mod render;
mod sim;
mod surface;

pub(crate) use surface::Surface;

/// Nombre de flocons préparés (le réglage de neige et le nombre d'écrans en utilisent une partie).
pub(crate) const FLAKE_POOL: usize = 1600;

pub(crate) struct App {
    /// Fenêtre cachée qui reçoit l'icône de notification et ses messages.
    pub(crate) hwnd: HWND,
    pub(crate) hinst: HINSTANCE,
    /// Un overlay par écran; tout le monde partage le même monde (coordonnées du bureau virtuel).
    pub(crate) surfaces: Vec<Surface>,
    pub(crate) dxgi_dev: IDXGIDevice,
    pub(crate) dxgi_factory: IDXGIFactory2,
    /// Origine de l'écran en cours de dessin (le monde est translaté de -origin).
    pub(crate) origin: std::cell::Cell<(f32, f32)>,
    /// Rectangle englobant tous les écrans, et rectangle de l'écran principal.
    pub(crate) vx: f32,
    pub(crate) vy: f32,
    pub(crate) pm: (f32, f32, f32, f32),
    /// Facteur de densité de neige (largeur totale des écrans / largeur de l'écran principal).
    pub(crate) density: f32,
    pub(crate) ctx: ID2D1DeviceContext,
    pub(crate) brush: ID2D1SolidColorBrush,
    pub(crate) pen: ID2D1SolidColorBrush,
    pub(crate) draw_failed: bool,
    /// Géométries des tas: recalculées quand leur forme change, au plus 5 fois par seconde.
    /// Plan de décoration de la barre des tâches (recalculé quand la longueur du terrain change).
    pub(crate) tb_plan: Vec<TbItem>,
    pub(crate) tb_plan_len: f32,
    pub(crate) pile_cache: RefCell<HashMap<isize, (u64, f32, Vec<(ID2D1PathGeometry, ID2D1PathGeometry)>)>>,
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
    pub(crate) dcomp: IDCompositionDevice,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) flakes: Vec<Flake>,
    pub(crate) rng: Rng,
    pub(crate) t: f32,
    pub(crate) active: bool,
    pub(crate) cfg: Settings,
    pub(crate) ticks: u32,
    pub(crate) last: Instant,
    pub(crate) last_fs_check: Instant,
    pub(crate) last_refresh: Instant,
    pub(crate) debris: Vec<Debris>,
    pub(crate) anchors: HashMap<isize, Anchor>,
    pub(crate) wins: Vec<WinInfo>,
}

thread_local! { pub(crate) static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

/// Vrai si l'erreur signifie que le périphérique graphique est perdu (il faut tout recréer).
pub(crate) fn is_device_lost(e: &windows::core::Error) -> bool {
    [DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DRIVER_INTERNAL_ERROR, D2DERR_RECREATE_TARGET].contains(&e.code())
}

impl App {
    /// Recrée tout (périphériques, overlays, sprites) après une perte du périphérique graphique.
    /// Le nouvel état est construit d'abord: en cas d'échec l'ancien reste en place et on réessaiera.
    pub(crate) fn recover_device(&mut self) -> Result<()> {
        let fresh = App::new(self.hwnd, self.hinst)?;
        let active = self.active;
        self.drop_surfaces();
        *self = fresh;
        self.active = active;
        self.apply_visibility();
        Ok(())
    }

    pub(crate) fn new(hwnd: HWND, hinst: HINSTANCE) -> Result<App> {
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

            let mut rng = Rng(0x2545F4914F6CDD1D);
            let flakes = (0..FLAKE_POOL)
                .map(|_| Flake { x: 0.0, y: 0.0, r: 1.5 + rng.next() * 3.5, speed: 30.0 + rng.next() * 60.0, phase: rng.next() * 6.28 })
                .collect();

            let salt = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1) | 1;
            let mut app = App {
                hwnd,
                hinst,
                surfaces: Vec::new(),
                dxgi_dev,
                dxgi_factory: factory,
                origin: std::cell::Cell::new((0.0, 0.0)),
                vx: 0.0,
                vy: 0.0,
                pm: (0.0, 0.0, 1920.0, 1080.0),
                density: 1.0,
                ctx,
                brush,
                pen,
                draw_failed: false,
                tb_plan: Vec::new(),
                tb_plan_len: 0.0,
                pile_cache: RefCell::new(HashMap::new()),
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
                dcomp,
                w: 1920.0,
                h: 1080.0,
                flakes,
                rng,
                t: 0.0,
                active: true,
                cfg: Settings::load(),
                ticks: 0,
                last: Instant::now(),
                last_fs_check: Instant::now() - std::time::Duration::from_secs(1),
                last_refresh: Instant::now(),
                debris: Vec::new(),
                anchors: HashMap::new(),
                wins: Vec::new(),
            };
            app.rebuild_surfaces()?;
            Ok(app)
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_loss_errors_are_recognized() {
        for hr in [DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET, DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DRIVER_INTERNAL_ERROR, D2DERR_RECREATE_TARGET] {
            assert!(is_device_lost(&windows::core::Error::from(hr)), "{hr:?}");
        }
    }

    #[test]
    fn ordinary_errors_are_not_device_loss() {
        for hr in [E_FAIL, E_OUTOFMEMORY, E_INVALIDARG] {
            assert!(!is_device_lost(&windows::core::Error::from(hr)), "{hr:?}");
        }
    }
}
