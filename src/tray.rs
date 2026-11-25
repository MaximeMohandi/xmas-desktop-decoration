//! Icône de la zone de notification, menu et fenêtre de l'Overlay.
use crate::prelude::*;

pub(crate) const WM_TRAY: u32 = WM_APP + 1;

// Demande de reconstruction des overlays (écran branché, débranché ou réorganisé).
thread_local! { pub(crate) static REBUILD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
// Message « TaskbarCreated »: diffusé quand l'explorateur redémarre (l'icône de la zone de notification a disparu).
thread_local! { pub(crate) static TASKBAR_CREATED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
pub(crate) const ID_TOGGLE: usize = 1001;
pub(crate) const ID_QUIT: usize = 1002;
pub(crate) const ID_SANTA: usize = 1003;
pub(crate) const ID_SLEIGH: usize = 1004;
pub(crate) const ID_SNOW0: usize = 1010; // 1010..=1013: aucune, légère, normale, forte
pub(crate) const ID_DECOR: usize = 1020;
pub(crate) const ID_AUTO: usize = 1021;
pub(crate) const ID_STARTUP: usize = 1022;
pub(crate) const ID_FREQ0: usize = 1030; // 1030..=1032: rare, normal, fréquent
pub(crate) const ID_PILE0: usize = 1040; // 1040..=1042: fine, normale, épaisse

/// Enregistre les classes (overlays + fenêtre cachée de l'icône) et crée la fenêtre cachée qui reçoit l'icône et ses messages.
pub(crate) unsafe fn create_main_window(hinst: HINSTANCE) -> Result<HWND> {
    // Classe des overlays (une fenêtre par écran, créées par l'application).
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(overlay_proc), hInstance: hinst, lpszClassName: w!("SneakyXmasOverlay"), ..Default::default() });
    TASKBAR_CREATED.with(|m| m.set(RegisterWindowMessageW(w!("TaskbarCreated"))));
    let main_class = w!("SneakyXmasMain");
    RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst, lpszClassName: main_class, ..Default::default() });
    CreateWindowExW(WS_EX_TOOLWINDOW, main_class, w!("sneakyXmas"), WS_POPUP, 0, 0, 0, 0, None, None, hinst, None)
}

pub(crate) fn add_tray(hwnd: HWND) {
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

pub(crate) fn remove_tray(hwnd: HWND) {
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

pub(crate) fn show_menu(hwnd: HWND, cfg: Settings) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let Ok(snow) = CreatePopupMenu() else { return };
        let chk = |on: bool| if on { MF_STRING | MF_CHECKED } else { MF_STRING };
        let names = [w!("Aucune"), w!("Légère"), w!("Normale"), w!("Forte")];
        for (i, n) in names.iter().enumerate() {
            let _ = AppendMenuW(snow, chk(cfg.snow == i), ID_SNOW0 + i, *n);
        }
        let _ = AppendMenuW(menu, chk(cfg.active), ID_TOGGLE, w!("Mode Noël"));
        let _ = AppendMenuW(menu, MF_POPUP, snow.0 as usize, w!("Neige"));
        let _ = AppendMenuW(menu, chk(cfg.decor), ID_DECOR, w!("Décorations"));
        let _ = AppendMenuW(menu, chk(cfg.auto_chars), ID_AUTO, w!("Père Noël et traîneau automatiques"));
        let Ok(freq) = CreatePopupMenu() else { return };
        for (i, n) in [w!("Rare"), w!("Normale"), w!("Fréquente")].iter().enumerate() {
            let _ = AppendMenuW(freq, chk(cfg.freq == i), ID_FREQ0 + i, *n);
        }
        let Ok(pile) = CreatePopupMenu() else { return };
        for (i, n) in [w!("Fine"), w!("Normale"), w!("Épaisse")].iter().enumerate() {
            let _ = AppendMenuW(pile, chk(cfg.pile == i), ID_PILE0 + i, *n);
        }
        let _ = AppendMenuW(menu, MF_POPUP, pile.0 as usize, w!("Neige accumulée"));
        let _ = AppendMenuW(menu, MF_POPUP, freq.0 as usize, w!("Fréquence des passages"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, ID_SANTA, w!("Lancer le Père Noël"));
        let _ = AppendMenuW(menu, MF_STRING, ID_SLEIGH, w!("Lancer le traîneau"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, chk(startup_enabled()), ID_STARTUP, w!("Lancer avec Windows"));
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quitter"));
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, None);
        // Recommandation Microsoft: sans ce message, le menu ne se referme pas toujours au clic ailleurs.
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }
}

/// Vrai si `msg` est le message « TaskbarCreated » enregistré (`registered` = 0: pas encore enregistré).
pub(crate) fn is_taskbar_created(msg: u32, registered: u32) -> bool {
    registered != 0 && msg == registered
}

/// Accès à l'application depuis un message, sans jamais paniquer: faux si elle est occupée (emprunt en cours dans `tick()`).
/// Une panique ici traverserait la frontière `extern "system"` et tuerait l'application sans trace.
fn with_app(f: impl FnOnce(&mut App)) -> bool {
    APP.with(|a| match a.try_borrow_mut() {
        Ok(mut g) => {
            if let Some(app) = g.as_mut() {
                f(app);
            }
            true
        }
        Err(_) => false,
    })
}

/// Commandes du menu qui agissent sur l'application.
fn handle_command(app: &mut App, id: usize) {
    match id {
        ID_TOGGLE => {
            app.active = !app.active;
            app.cfg.active = app.active;
            app.cfg.save();
            app.apply_visibility();
        }
        ID_DECOR => {
            app.cfg.decor = !app.cfg.decor;
            app.cfg.save();
        }
        ID_AUTO => {
            app.cfg.auto_chars = !app.cfg.auto_chars;
            app.cfg.save();
        }
        ID_FREQ0..=1032 => {
            app.cfg.freq = id - ID_FREQ0;
            app.cfg.save();
        }
        ID_PILE0..=1042 => {
            app.cfg.pile = id - ID_PILE0;
            app.cfg.save();
        }
        ID_SNOW0..=1013 => {
            app.cfg.snow = id - ID_SNOW0;
            app.cfg.save();
        }
        ID_SANTA => app.next_santa = 0.0,
        ID_SLEIGH => app.next_sleigh = 0.0,
        _ => {}
    }
}

pub(crate) unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // L'explorateur a redémarré (plantage, « Redémarrer l'explorateur »): on remet l'icône, sinon plus de menu ni de « Quitter ».
    if is_taskbar_created(msg, TASKBAR_CREATED.with(|m| m.get())) {
        add_tray(hwnd);
        return LRESULT(0);
    }
    match msg {
        WM_TRAY => {
            if (lp.0 as u32) == WM_RBUTTONUP || (lp.0 as u32) == WM_CONTEXTMENU {
                let mut cfg = None;
                with_app(|app| cfg = Some(app.cfg));
                if let Some(cfg) = cfg {
                    show_menu(hwnd, cfg);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wp.0 & 0xFFFF {
                ID_STARTUP => set_startup(!startup_enabled()),
                ID_QUIT => {
                    let _ = DestroyWindow(hwnd);
                }
                id => {
                    // Application occupée (message reçu pendant `tick()`): on remet la commande en file, elle sera traitée juste après.
                    if !with_app(|app| handle_command(app, id)) {
                        let _ = PostMessageW(hwnd, msg, wp, lp);
                    }
                }
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            REBUILD.with(|r| r.set(true));
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_DESTROY => {
            remove_tray(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Fenêtre d'un overlay: ne fait rien (insensible aux clics); sa destruction ne quitte pas l'application.
pub(crate) unsafe extern "system" fn overlay_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskbar_created_detection() {
        assert!(is_taskbar_created(0xC123, 0xC123));
        assert!(!is_taskbar_created(0xC123, 0xC124));
        assert!(!is_taskbar_created(0, 0), "message 0 jamais confondu avec un message non enregistré");
    }
}
