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

        let mut app = App::new(hwnd, w, h)?;
        app.active = app.cfg.active;
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

