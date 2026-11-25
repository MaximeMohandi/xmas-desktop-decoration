//! sneakyXmas: Overlay de Noël pour Windows 11 (neige, décors, père Noël).
#![windows_subsystem = "windows"]

mod app;
mod characters;
mod desktop;
mod prelude;
mod scenes;
mod settings;
mod snow;
mod sprites;
mod tray;
mod util;

use prelude::*;

fn main() {
    if let Err(e) = run() {
        let _ = std::fs::write("sneakyxmas.log", format!("{e:?}\n"));
    }
}

fn run() -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // Instance unique: si une autre tourne déjà, on s'efface.
        let _guard = CreateMutexW(None, true, w!("Local\\sneakyXmas-instance"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        let hinst: HINSTANCE = GetModuleHandleW(None)?.into();
        // Classe des overlays (une fenêtre par écran, créées par l'application) et de la fenêtre cachée de l'icône.
        RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(overlay_proc), hInstance: hinst, lpszClassName: w!("SneakyXmasOverlay"), ..Default::default() });
        let main_class = w!("SneakyXmasMain");
        RegisterClassW(&WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hinst, lpszClassName: main_class, ..Default::default() });
        let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, main_class, w!("sneakyXmas"), WS_POPUP, 0, 0, 0, 0, None, None, hinst, None)?;

        let mut app = App::new(hwnd, hinst)?;
        app.active = app.cfg.active;
        app.apply_visibility();
        APP.with(|a| *a.borrow_mut() = Some(app));
        add_tray(hwnd);

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

