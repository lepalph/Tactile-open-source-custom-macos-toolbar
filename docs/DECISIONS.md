# Décisions

Les choix qui surprennent à la lecture du code, avec leur raison. Avant d'en défaire un, lis pourquoi il a été pris.

## 1. Développement en salle blanche

**Décision.** Tactile est écrit à partir des API d'Apple, du code open source de MTMR (licence MIT) et du comportement de BetterTouchTool observé de l'extérieur. BetterTouchTool n'a jamais été désassemblé.

**Pourquoi.** BTT est un logiciel commercial dont la licence interdit la rétro-ingénierie ; en recopier le code serait de la contrefaçon. Tout ce dont Tactile a besoin est documenté ou visible dans MTMR.

## 2. Rust et `msg_send!` brut plutôt que des bindings typés

**Décision.** La couche AppKit utilise objc2 avec des `msg_send!` sur des pointeurs `*mut AnyObject`, sans les crates `objc2-app-kit`.

**Pourquoi.** Une bonne partie des appels sont privés (barre système, Control Strip) et n'existent dans aucun binding. Mélanger les deux styles rendrait le code plus dur à suivre. Le prix : moins de vérification à la compilation, compensé par la vérification des signatures à l'exécution d'objc2 et par les captures.

## 3. Sous-barres système au lieu des popovers d'AppKit

**Décision.** Un slider ou un groupe ouvre une nouvelle barre système avec un ✕, au lieu d'un `NSPopoverTouchBarItem`.

**Pourquoi.** Les popovers ne s'ouvrent pas dans une barre système (vérifié). MTMR a le même problème et ses groupes s'en trouvaient cassés (pas de sortie, actions muettes). Les sous-barres ont été vérifiées en capture.

## 4. `media-control` pour la musique

**Décision.** Le titre en cours et les commandes de lecture passent par l'outil externe `media-control` (Homebrew), pas par MediaRemote.

**Pourquoi.** Depuis macOS 15.4, MediaRemote refuse aux apps non autorisées de lire ce qui joue, et ses commandes répondent « vrai » sans effet (vérifié). `media-control` passe par un binaire qu'Apple autorise encore. Il reste optionnel : sans lui, le reste de la barre marche.

## 5. Config en TOML, clés en français, relue par sondage

**Décision.** La barre se décrit dans un fichier TOML aux clés françaises. Un fil relit sa date de modification chaque seconde.

**Pourquoi.** Un fichier texte se versionne, se partage et se corrige à la main. Le sondage tient en vingt lignes de bibliothèque standard, contre une dépendance pour FSEvents, pour une seconde de délai au pire. Les clés françaises suivent la règle du projet ; le README anglais en donne la traduction. Un éditeur visuel est prévu par-dessus.

## 6. App universelle

**Décision.** `Tactile.app` contient un binaire arm64 et un binaire x86_64.

**Pourquoi.** La plupart des MacBook Pro à Touch Bar sont des Intel (2016 à 2020). Le binaire Intel a été vérifié sous Rosetta ; il n'a pas encore tourné sur un vrai Mac Intel.

## 7. Pas de notarisation

**Décision.** L'app est signée ad hoc, pas notarisée.

**Pourquoi.** La notarisation demande un compte développeur Apple payant. Conséquence pour l'utilisateur : le premier lancement passe par clic droit → Ouvrir.
