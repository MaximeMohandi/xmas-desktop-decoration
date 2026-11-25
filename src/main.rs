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
#[cfg(test)]
mod testkit;
#[cfg(test)]
mod tests;

use prelude::*;

fn main() {
    // Sans console (windows_subsystem), une panique disparaîtrait sans trace: on l'écrit dans le journal.
    std::panic::set_hook(Box::new(|info| {
        let _ = std::fs::write("sneakyxmas.log", format!("panic: {info}\n"));
    }));
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
        let hwnd = create_main_window(hinst)?;

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

