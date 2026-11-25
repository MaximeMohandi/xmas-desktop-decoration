# sneakyXmas — notes de reprise pour Claude Code

Application résidente Windows 11 qui dessine un **overlay de Noël** (neige qui s'accumule, village sur la barre des tâches,
Père Noël, traîneau) par-dessus le bureau. Rust + crate `windows` 0.58 (Direct2D / DirectComposition). Usage perso et amis.
Dépôt public : https://github.com/MaximeMohandi/xmas-desktop-decoration — tags `v1` (un écran), `v2` (multi-écrans, Release publiée).
Langue de travail avec l'utilisateur : **français** (UI, README, commits).

Lire d'abord : `README.md`, `GLOSSARY.md` (vocabulaire : Overlay, Décoration, Ancre, Tas, Schéma de sons…), `docs/adr/`.

## État

Fait : overlay un par écran, neige + tas lissés, décor fixe par fenêtre, barre des tâches = un seul « terrain » multi-écrans
décoré par un plan d'urbanisme, Père Noël, traîneau, plein écran par écran, menu de la zone de notification, réglages
mémorisés (`sneakyxmas.ini`), démarrage avec Windows (HKCU Run), instance unique, README FR/EN avec captures, Release v2.

**À faire : les sons** (voir plus bas). Décision en attente : l'utilisateur n'a pas encore tranché la source des sons
(synthétisés par moi / fichiers fournis / mix). La question a été interrompue : la reposer.

## Construire et lancer

- Windows 11, Rust (MSVC) + Visual Studio Build Tools. `cargo build --release`.
- Historique : le développement s'est fait depuis **WSL2** en appelant `cargo.exe` sur `/mnt/c/...` (pas de linker Windows dans WSL) :
  `cmd.exe /c 'cd /d C:\projects\perso\sneakyXmas && C:\Users\<user>\.cargo\bin\cargo.exe build --release --target-dir target-new'`
  (`target-new` évite les verrous du `target\release` quand l'exe tourne).
- Déployer pour test : copier l'exe dans `dist\` (ignoré par git), tuer l'ancienne instance **avant** de recompiler
  (`Stop-Process -Force` sur `sneakyxmas` non terminé), relancer avec `-WorkingDirectory` sur `dist` (le `.ini` et les logs y vivent).
  Un second lancement s'arrête seul (mutex) : toujours vérifier le PID et `sneakyxmas.log` (écrit seulement en cas d'erreur).
- L'utilisateur valide **visuellement** ; on ne peut pas voir l'écran. Dire clairement ce qui n'a pas pu être vérifié.

## Tests

`cargo test` (mono-thread forcé par `.cargo/config.toml`). Session Windows **interactive** requise (écran + GPU) : les tests d'intégration
(`src/tests.rs`, banc d'essai `src/testkit.rs`) créent de vrais overlays et de vraies fenêtres, visibles une fraction de seconde.
Depuis WSL : `cmd.exe /c 'cd /d C:\projects\perso\sneakyXmas && C:\Users\<user>\.cargo\bin\cargo.exe test --target-dir target-new'`.
- Tests unitaires à côté du code (`desktop.rs`, `tray.rs`, `app/mod.rs`) : délai de grâce des ancres, règle du plein écran, détection de
  perte du périphérique, message `TaskbarCreated`.
- Régressions couvertes : `WM_SETTINGCHANGE` (fenêtre qui se rafraîchit) ne reconstruit pas les overlays ; reconstruction réessayée après échec ;
  fenêtre qui clignote garde sa décoration ; message reçu pendant `tick()` ni paniqué ni perdu ; reprise après perte du GPU ;
  fenêtre maximisée ordinaire + barre auto-masquée ≠ plein écran.
- Règle : tout bug corrigé reçoit un test qui échoue sans le correctif (vérifier en le retirant un instant).
- Non testable automatiquement : l'aspect visuel, la vraie perte de GPU (seule la reprise est testée), un vrai redémarrage de l'explorateur.

## Architecture (`src/`)

| Fichier | Rôle |
|---|---|
| `main.rs` | démarrage, mutex d'instance unique, boucle de messages, classes de fenêtres |
| `app/mod.rs` | `App` (état), création des périphériques D3D/D2D/DComp |
| `app/surface.rs` | un overlay par écran (fenêtre + swap chain + visuel), reconstruction au changement d'écrans |
| `app/sim.rs` | simulation : fenêtres, ancres, neige, tas, personnages, plein écran |
| `app/render.rs` | dessin d'une image par écran (monde translaté de -origine) |
| `desktop.rs` | `WinInfo`, `Anchor`, `covered()` (recouvrement par z-order), énumération, plein écran |
| `snow.rs` / `scenes.rs` / `sprites.rs` | flocons et couche de départ / scènes + plan de la barre / chargement PNG (WIC) |
| `settings.rs` / `tray.rs` | ini + registre / icône, menu, fenêtres |
| `testkit.rs` / `tests.rs` | (tests seulement) banc d'essai et tests d'intégration |

Concepts clés : monde partagé en **coordonnées du bureau virtuel** (négatives possibles) ; ancres : clé `0` = barre principale,
clé négative = barre d'un autre écran, clé positive = fenêtre (hwnd). `covered(wins, z, x, y)` masque ce qu'une fenêtre située
au-dessus cache. Les décors sont dessinés **après** les tas (jamais enfouis).

## Pièges déjà rencontrés (ne pas refaire)

- L'overlay est **au-dessus** de la barre des tâches : tout doit être masqué explicitement derrière les fenêtres. Tester la couverture
  **dans** la barre (`a.t + 2`), pas au-dessus (sinon Père Noël masqué quand une fenêtre prend le focus).
- Performance : ne pas recréer de géométries Direct2D à chaque image (CPU ×4). Cache des tas, reconstruction ≤ 5 Hz.
  `ID2D1SpriteBatch::DrawSpriteBatch` a échoué (`D2DERR_WRONG_STATE`) : abandonné. Mesures très bruitées.
- Une erreur dans `draw()` ne doit jamais faire tourner la boucle à vide (déjà protégé : log + sleep).
- Détection plein écran : toute fenêtre recouvrant un écran, pas seulement celle au premier plan (multi-écrans).
- `target\release\sneakyxmas.exe` peut rester verrouillé par un processus zombie : utiliser `target-new`.
- Un processus qui tourne verrouille l'exe : tuer avant de recompiler.

## Conventions et préférences de l'utilisateur

- **Style visuel** : illustration lisse et moderne (pas de pixel art, pas de formes vectorielles codées à la main). Images
  générées par IA par l'utilisateur, source dans `reference/` (non versionné, exclu par `.gitignore`), PNG dans `assets/`.
- **Décorer comme une maison / un plan d'urbanisme** : retenue, groupes impairs, asymétrie, repères, vides et clairières.
  Pas de guirlandes. Décor de la barre stable d'un lancement à l'autre ; décor d'une fenêtre tiré une fois et conservé.
- Git : identité **locale** au dépôt (jamais la config globale). Commits en français avec la ligne `Co-Authored-By` de Claude.
  **Ne pousser que sur demande explicite** (« push »). Dépôt public : ne rien y mettre de privé (captures floutées, pas de chemins perso).
  Push depuis WSL impossible (pas d'identifiants) : utiliser le Git de Windows via
  `cmd.exe /c 'cd /d C:\projects\perso\sneakyXmas && git push origin main'`.
- La Release GitHub est créée **à la main** par l'utilisateur (pas de `gh`) : lui fournir le zip + notes. Zip = exe compilé depuis le tag,
  README FR/EN, LICENSE, `docs/screenshots/`.
- Licence MIT du dépôt, mais le README dit que les images ne sont pas libres de droits : **contradiction à clarifier**
  (préciser que la MIT ne couvre que le code, ou retirer les images).

## Prochaines étapes

1. **Sons** (HKCU `AppEvents\Schemes`) : schéma de Noël (WAV libres de droits ou synthétisés), sauvegarde du schéma d'origine sur disque,
   restauration automatique à la désactivation et à la fermeture/au plantage, option `--restore-sounds`, ne pas toucher au son de démarrage.
   Entrées de menu : activer/désactiver les sons. Mettre à jour le README (section « Au programme ») et créer un tag `v3`.
2. Tester le branchement/débranchement d'un écran (reconstruction des overlays, non testé).
3. Cas du masquage automatique de la barre des tâches (une fenêtre maximisée recouvre tout l'écran → overlay masqué).
4. Taille des sprites selon la densité (DPI) de chaque écran ; barre des tâches non située en bas.
5. Optionnel : interpolation moins coûteuse pour les décors ; utiliser des icônes encore inutilisées (glaçons, enseigne…).
