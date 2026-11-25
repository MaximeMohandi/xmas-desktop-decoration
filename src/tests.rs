//! Tests d'intégration (vraies fenêtres Win32, vrai Direct3D). Voir la section « Tests » de CLAUDE.md.
//! Lancer avec `cargo test -- --test-threads=1` dans une session Windows interactive.
use crate::prelude::*;
use crate::testkit::Harness;

fn surface_hwnds(h: &Harness) -> Vec<isize> {
    h.app(|a| a.surfaces.iter().map(|s| s.hwnd.0 as isize).collect())
}

/// Régression: une fenêtre qui se rafraîchit fait diffuser `WM_SETTINGCHANGE` par Windows.
/// Cela ne doit PAS reconstruire les overlays (neige perdue, décors retirés au hasard, Père Noël disparu).
#[test]
fn settingchange_without_layout_change_keeps_overlays_and_state() {
    let mut h = Harness::new();
    let win = h.open_window(200, 300, 400, 300);
    let key = win.0 as isize;
    h.app(|a| a.refresh_windows());
    let (seed, kind) = h.app(|a| {
        let an = a.anchors.get(&key).expect("la fenêtre de test doit avoir une ancre");
        (an.seed, an.kind)
    });
    let before = surface_hwnds(&h);
    h.app(|a| a.santa_x = Some(500.0));

    for _ in 0..5 {
        h.send(WM_SETTINGCHANGE, 0, 0);
        h.app(|a| a.tick());
    }

    assert_eq!(surface_hwnds(&h), before, "les overlays ont été reconstruits sans changement d'écran");
    h.app(|a| {
        let an = a.anchors.get(&key).expect("l'ancre a disparu");
        assert_eq!((an.seed, an.kind), (seed, kind), "la décoration de la fenêtre a été retirée au hasard");
        assert!(a.santa_x.is_some(), "le Père Noël a été réinitialisé");
    });
}

/// Un vrai changement de disposition des écrans doit, lui, reconstruire les overlays.
#[test]
fn real_layout_change_rebuilds_overlays() {
    let h = Harness::new();
    let before = surface_hwnds(&h);
    h.app(|a| a.surfaces[0].rect.2 += 100.0); // l'overlay ne correspond plus à l'écran réel
    h.send(WM_DISPLAYCHANGE, 0, 0);
    h.app(|a| a.tick());
    assert_ne!(surface_hwnds(&h), before, "le changement d'écran n'a pas reconstruit les overlays");
    assert!(!h.app(|a| a.layout_changed()), "après reconstruction la disposition doit correspondre");
}

/// Régression: un échec de reconstruction laissait l'appli sans overlay jusqu'au redémarrage.
/// Sans overlay, la condition de nouvel essai doit rester vraie; un `WM_DISPLAYCHANGE` relance alors la reconstruction.
#[test]
fn rebuild_is_retried_after_surfaces_were_lost() {
    let h = Harness::new();
    h.app(|a| a.drop_surfaces());
    assert!(!h.app(|a| a.is_rendering()));
    assert!(h.app(|a| a.layout_changed()), "sans overlay, un nouvel essai doit rester nécessaire");
    h.send(WM_DISPLAYCHANGE, 0, 0);
    h.app(|a| a.tick());
    assert!(!h.app(|a| a.surfaces.is_empty()), "les overlays n'ont pas été recréés");
}

/// Test de fumée: quelques images complètes (simulation + dessin + Present) sans erreur.
#[test]
fn ticks_draw_without_error() {
    let mut h = Harness::new();
    h.open_window(150, 250, 500, 350);
    for _ in 0..20 {
        h.app(|a| a.tick());
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(!h.app(|a| a.draw_failed), "draw() a échoué");
    assert!(h.app(|a| a.draw()).is_ok());
}

// ---- Fenêtre qui clignote (invisible un instant): sa décoration ne doit pas être retirée au hasard ----

#[test]
fn flickering_window_keeps_its_decoration() {
    let mut h = Harness::new();
    let win = h.open_window(200, 300, 400, 300);
    let key = win.0 as isize;
    h.app(|a| a.refresh_windows_dt(0.1));
    let id = h.app(|a| {
        let an = &a.anchors[&key];
        (an.seed, an.kind, an.variant, an.left)
    });
    h.hide(win);
    h.app(|a| a.refresh_windows_dt(0.2));
    h.app(|a| {
        assert!(a.anchors.contains_key(&key), "l'ancre a été supprimée dès la première absence");
        assert!(!a.anchors[&key].present());
        assert!(a.live_anchors().all(|(k, _)| *k != key), "une ancre absente ne doit ni se dessiner ni recevoir de neige");
    });
    h.show(win);
    h.app(|a| a.refresh_windows_dt(0.2));
    h.app(|a| {
        let an = &a.anchors[&key];
        assert!(an.present());
        assert_eq!((an.seed, an.kind, an.variant, an.left), id, "la décoration a été retirée au hasard");
    });
}

#[test]
fn closed_window_loses_its_anchor_after_the_grace_period() {
    let mut h = Harness::new();
    let win = h.open_window(200, 300, 400, 300);
    let key = win.0 as isize;
    h.app(|a| a.refresh_windows_dt(0.1));
    h.hide(win);
    for _ in 0..4 {
        h.app(|a| a.refresh_windows_dt(0.5));
    }
    assert!(!h.app(|a| a.anchors.contains_key(&key)), "l'ancre d'une fenêtre disparue doit finir par être supprimée");
}

// ---- Un message qui arrive pendant que l'appli est occupée ne doit ni paniquer ni être perdu ----

#[test]
fn command_received_while_app_is_busy_is_deferred_not_lost() {
    let h = Harness::new();
    h.app(|a| a.next_santa = 99.0);
    APP.with(|cell| {
        let _busy = cell.borrow_mut(); // comme pendant tick()
        h.send(WM_COMMAND, ID_SANTA, 0);
        h.send(WM_TRAY, 1, WM_RBUTTONUP as isize); // ne doit pas ouvrir de menu ni paniquer
    });
    h.pump(); // la commande remise en file s'exécute maintenant
    assert_eq!(h.app(|a| a.next_santa), 0.0, "la commande du menu a été perdue");
}

// ---- Explorateur redémarré ----

#[test]
fn taskbar_created_message_is_registered_and_handled() {
    let h = Harness::new();
    let m = TASKBAR_CREATED.with(|m| m.get());
    assert_ne!(m, 0, "message TaskbarCreated non enregistré");
    h.send(m, 0, 0); // ré-ajoute l'icône sans paniquer
}

// ---- Perte du périphérique graphique ----

#[test]
fn device_recovery_rebuilds_everything_and_keeps_drawing() {
    let h = Harness::new();
    let before = surface_hwnds(&h);
    h.app(|a| a.recover_device().expect("recover_device"));
    assert_ne!(surface_hwnds(&h), before, "les overlays n'ont pas été recréés");
    assert!(h.app(|a| a.is_rendering()));
    for _ in 0..5 {
        h.app(|a| a.tick());
    }
    assert!(h.app(|a| a.draw()).is_ok(), "le dessin échoue après la reprise");
}
