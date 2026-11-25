# sneakyXmas

Une application résidente pour Windows 11 qui habille le bureau, la barre des tâches et l'explorateur d'animations de Noël (neige, sapins, pères Noël) et de sons de fête. Elle est pensée pour un usage personnel, partageable entre amis.

## Language

**Mode Noël**:
L'état global « activé » de l'application: l'Overlay est visible et, si l'utilisateur l'a choisi, le Schéma de sons de Noël est appliqué.
_Avoid_: Thème, skin, saison

**Overlay**:
La fenêtre transparente, toujours au premier plan et insensible aux clics, sur laquelle sont dessinées les Décorations par-dessus la barre des tâches, le bureau et l'explorateur.
_Avoid_: Calque, surcouche, injection

**Décoration**:
Un élément visuel animé affiché sur l'Overlay (flocon, sapin, père Noël, guirlande).
_Avoid_: Sprite, effet, animation

**Ancre**:
Une surface réelle de Windows (la barre des tâches, une fenêtre ouverte) à laquelle une Décoration est attachée et qu'elle suit.
_Avoid_: Support, cible, point d'accroche

**Tas**:
La neige accumulée sur le bord supérieur d'une Ancre. Il se détache et tombe quand l'Ancre bouge.
_Avoid_: Pile, accumulation, congère

**Schéma de sons**:
L'ensemble des sons associés aux événements système de Windows (démarrage, notification, erreur, etc.).
_Avoid_: Thème sonore, pack de sons

**Schéma de Noël**:
Le Schéma de sons festif fourni par l'application et appliqué en Mode Noël.
_Avoid_: Sons de Noël

**Schéma d'origine**:
Le Schéma de sons de l'utilisateur, sauvegardé avant l'application du Schéma de Noël et restauré à la désactivation.
_Avoid_: Sons par défaut, backup

**Mode plein écran**:
La situation où une application (jeu, film) occupe tout l'écran. L'Overlay doit alors être masqué pour ne pas la perturber.
_Avoid_: Fullscreen, mode jeu
