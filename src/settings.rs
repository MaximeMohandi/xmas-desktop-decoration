//! Réglages mémorisés et lancement avec Windows.
use crate::prelude::*;

/// Réglages mémorisés dans `sneakyxmas.ini` (à côté de l'exécutable).
#[derive(Clone, Copy)]
pub(crate) struct Settings {
    pub(crate) active: bool,
    pub(crate) snow: usize, // 0 aucune, 1 légère, 2 normale, 3 forte
    pub(crate) decor: bool,
    pub(crate) auto_chars: bool, // Père Noël et traîneau qui passent d'eux-mêmes
    pub(crate) freq: usize, // fréquence des passages: 0 rare, 1 normal, 2 fréquent
    pub(crate) pile: usize, // épaisseur max de la neige accumulée: 0 fine, 1 normale, 2 épaisse
}
pub(crate) const SNOW_COUNT: [usize; 4] = [0, 90, 220, 400];
pub(crate) const FREQ_MUL: [f32; 3] = [2.0, 1.0, 0.4]; // multiplie le délai entre deux passages
pub(crate) const PILE_MAXS: [f32; 3] = [6.0, 12.0, 20.0];

impl Settings {
    pub(crate) fn path() -> std::path::PathBuf {
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("sneakyxmas.ini"))).unwrap_or_else(|| "sneakyxmas.ini".into())
    }
    pub(crate) fn load() -> Settings {
        let mut c = Settings { active: true, snow: 2, decor: true, auto_chars: true, freq: 1, pile: 1 };
        if let Ok(txt) = std::fs::read_to_string(Self::path()) {
            for line in txt.lines() {
                let Some((k, v)) = line.split_once('=') else { continue };
                let (k, v) = (k.trim(), v.trim());
                let b = v == "1" || v.eq_ignore_ascii_case("true");
                match k {
                    "active" => c.active = b,
                    "snow" => c.snow = v.parse::<usize>().unwrap_or(2).min(3),
                    "decor" => c.decor = b,
                    "auto_chars" => c.auto_chars = b,
                    "freq" => c.freq = v.parse::<usize>().unwrap_or(1).min(2),
                    "pile" => c.pile = v.parse::<usize>().unwrap_or(1).min(2),
                    _ => {}
                }
            }
        }
        c
    }
    pub(crate) fn save(&self) {
        let txt = format!("active={}\nsnow={}\ndecor={}\nauto_chars={}\nfreq={}\npile={}\n", self.active as u8, self.snow, self.decor as u8, self.auto_chars as u8, self.freq, self.pile);
        let _ = std::fs::write(Self::path(), txt);
    }
}

pub(crate) const RUN_KEY: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

pub(crate) fn startup_enabled() -> bool {
    unsafe {
        let mut k = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, KEY_READ, &mut k).is_err() {
            return false;
        }
        let r = RegQueryValueExW(k, w!("sneakyXmas"), None, None, None, None).is_ok();
        let _ = RegCloseKey(k);
        r
    }
}

pub(crate) fn set_startup(on: bool) {
    unsafe {
        let mut k = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, KEY_WRITE, &mut k).is_err() {
            return;
        }
        if on {
            if let Ok(exe) = std::env::current_exe() {
                let v: Vec<u16> = format!("\"{}\"", exe.display()).encode_utf16().chain(std::iter::once(0)).collect();
                let bytes = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2);
                let _ = RegSetValueExW(k, w!("sneakyXmas"), 0, REG_SZ, Some(bytes));
            }
        } else {
            let _ = RegDeleteValueW(k, w!("sneakyXmas"));
        }
        let _ = RegCloseKey(k);
    }
}

