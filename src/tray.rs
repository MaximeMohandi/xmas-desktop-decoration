//! Icône de la zone de notification, menu et fenêtre de l'Overlay.
use crate::prelude::*;

pub(crate) const WM_TRAY: u32 = WM_APP + 1;
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
        let _ = DestroyMenu(menu);
    }
}

pub(crate) unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            if (lp.0 as u32) == WM_RBUTTONUP || (lp.0 as u32) == WM_CONTEXTMENU {
                let cfg = APP.with(|a| a.borrow().as_ref().map(|x| x.cfg));
                if let Some(cfg) = cfg {
                    show_menu(hwnd, cfg);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wp.0 & 0xFFFF {
                ID_TOGGLE => APP.with(|a| {
                    if let Some(app) = a.borrow_mut().as_mut() {
                        app.active = !app.active;
                        app.cfg.active = app.active;
                        app.cfg.save();
                        app.apply_visibility();
                    }
                }),
                id @ (ID_DECOR | ID_AUTO | ID_SNOW0..=1013 | ID_FREQ0..=1032 | ID_PILE0..=1042) => APP.with(|a| {
                    if let Some(app) = a.borrow_mut().as_mut() {
                        match id {
                            ID_DECOR => app.cfg.decor = !app.cfg.decor,
                            ID_AUTO => app.cfg.auto_chars = !app.cfg.auto_chars,
                            ID_FREQ0..=1032 => app.cfg.freq = id - ID_FREQ0,
                            ID_PILE0..=1042 => app.cfg.pile = id - ID_PILE0,
                            _ => app.cfg.snow = id - ID_SNOW0,
                        }
                        app.cfg.save();
                    }
                }),
                ID_STARTUP => set_startup(!startup_enabled()),
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

