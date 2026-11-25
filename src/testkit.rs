//! Banc d'essai des tests d'intégration: une vraie fenêtre principale, une vraie `App` (Direct3D, un overlay par écran)
//! et des fenêtres de test réelles. Demande une session Windows interactive (écran + GPU) et `--test-threads=1`.
use crate::prelude::*;

pub(crate) struct Harness {
    pub(crate) hwnd: HWND,
    windows: Vec<HWND>,
}

impl Harness {
    pub(crate) fn new() -> Harness {
        unsafe {
            let hinst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
            let hwnd = create_main_window(hinst).expect("fenêtre principale");
            let mut app = App::new(hwnd, hinst).expect("App::new (session Windows avec écran et GPU requise)");
            app.active = true;
            app.apply_visibility();
            APP.with(|a| *a.borrow_mut() = Some(app));
            Harness { hwnd, windows: Vec::new() }
        }
    }

    /// Accès à l'application, comme le fait la boucle principale.
    pub(crate) fn app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        APP.with(|a| f(a.borrow_mut().as_mut().expect("App absente")))
    }

    /// Traite les messages en attente (équivalent d'un tour de la boucle principale).
    pub(crate) fn pump(&self) {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    /// Ouvre une vraie fenêtre de premier niveau, visible, sans prendre le focus.
    pub(crate) fn open_window(&mut self, x: i32, y: i32, w: i32, h: i32) -> HWND {
        unsafe {
            let hinst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
            let win = CreateWindowExW(WS_EX_NOACTIVATE, w!("STATIC"), w!("sneakyXmas-test"), WS_POPUP | WS_BORDER, x, y, w, h, None, None, hinst, None)
                .expect("fenêtre de test");
            let _ = ShowWindow(win, SW_SHOWNOACTIVATE);
            self.windows.push(win);
            win
        }
    }

    pub(crate) fn hide(&self, win: HWND) {
        unsafe {
            let _ = ShowWindow(win, SW_HIDE);
        }
    }

    pub(crate) fn show(&self, win: HWND) {
        unsafe {
            let _ = ShowWindow(win, SW_SHOWNOACTIVATE);
        }
    }

    /// Diffuse un message système à la fenêtre principale (appel synchrone, comme Windows le fait).
    pub(crate) fn send(&self, msg: u32, wp: usize, lp: isize) {
        unsafe {
            SendMessageW(self.hwnd, msg, WPARAM(wp), LPARAM(lp));
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe {
            for w in self.windows.drain(..) {
                let _ = DestroyWindow(w);
            }
            if let Some(mut app) = APP.with(|a| a.borrow_mut().take()) {
                app.drop_surfaces();
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
