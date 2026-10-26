# Retour 0.3 · Inès, accessibilité (clavier, VoiceOver, zoom)

## Contexte

Inès, 45 ans, juriste, malvoyante. Elle utilise VoiceOver et le Zoom de macOS, ne touche jamais la souris et travaille sur un MacBook Pro à Touch Bar. Elle veut régler sa barre avec la fenêtre « Personnaliser ma barre ».

Méthode : éditeur seul (`tactile --personnaliser`, build release de `main`), `HOME` factice, copie de `exemples/barre.toml`, thèmes forcés `TACTILE_APPARENCE=clair|sombre` et pilote `TACTILE_EDITEUR_JS`. Le pilote :
- simule le clavier (`focus()`, `keydown` ←/→, ⌥←/→, Entrée, Retour arrière) ;
- lit l'élément actif après chaque geste ;
- calcule les noms accessibles (`aria-labelledby` > `aria-label` > `<label for>` > texte) ;
- mesure les contrastes avec `getComputedStyle`, en composant les fonds et les opacités jusqu'au `body` ;
- mesure la taille des cibles avec `getBoundingClientRect` ;
- double toutes les tailles de texte, puis applique un `zoom: 2`.

Ni la barre ni `scripts/verifier.sh` n'ont été lancés. Critères : WCAG 2.2 AA (référence 07 du skill ui-ux-director).

Limites :
- Un `keydown` synthétique ne déclenche pas les actions par défaut du navigateur (Tab natif, Échap natif d'un `<dialog>`). J'ai donc déduit l'ordre de tabulation de l'ordre du DOM et des `tabindex`, et fermé les dialogues avec `close()`.
- VoiceOver n'a pas été écouté : l'arbre d'accessibilité est déduit des attributs.
- Fenêtre masquée, WebKit suspend les minuteurs de la page. Le pilote se réveille par des allers-retours `verifierApp` → `appVerifiee`. C'est à savoir pour les futurs pilotes.

Captures (dans `captures/`) :
- `ines-anneau-focus-clair.png` et `ines-anneau-focus-sombre.png` : la règle `:focus-visible` de la ligne 56, rendue sur un bouton de la palette et sur « Exporter… » ;
- `ines-interrupteur-eteint-clair.png` : interrupteur éteint ;
- `ines-texte-200-clair.png` : texte doublé ;
- `ines-dialogue-import-sombre.png` : dialogue d'import.

## Parcours clavier

Ordre de tabulation mesuré, sans élément choisi : FR, EN, Importer…, Exporter…, (Revenir, désactivé), Enregistrer, **un seul arrêt** dans l'aperçu (tabindex itinérant, `editeur.html:808`), interrupteur « Cacher la bande… », 10 boutons « Ajouter », 3 « Barres toutes faites ». Une fois un élément choisi, on passe ensuite par ◀, ▶, Supprimer, Texte, Nom du symbole, Aucun symbole, Recherche, **48 boutons de symboles**, Choisir une image…, les couleurs, la taille, Afficher, la largeur et les deux menus d'action.

L'ordre est logique : il suit l'ordre visuel, de haut en bas puis de gauche à droite. Le parcours bute pourtant en cinq endroits :

| Geste | Résultat mesuré | Verdict |
|---|---|---|
| ←/→ dans l'aperçu | sélection et focus passent à l'élément voisin (`div.el.sel[data-i=1]`) | bon |
| ⌥→ dans l'aperçu | l'élément est déplacé, le focus le suit (`data-i=2`) | bon, mais rien dans l'interface ne le dit |
| ◀ / ▶ du panneau | l'élément est déplacé, le focus **saute dans l'aperçu** (`div.el.sel`) | acceptable |
| **Entrée** sur un élément de l'aperçu | focus perdu, `document.activeElement` = `body` | **bloquant** |
| Clic sur un élément de l'aperçu | focus perdu (`body`) | gênant |
| Clic sur un symbole de la grille | focus perdu (`body`) | **bloquant** |
| Supprimer (panneau) | focus perdu (`body`) | **bloquant** |
| Retour arrière sur un élément de l'aperçu | supprimé **sans confirmation ni annulation** (9 → 8 éléments), focus sur le premier élément | risque de perte |
| Ajouter (palette) | l'élément est ajouté et choisi, le focus reste sur le bouton de la palette | bon |
| Recherche de symbole, champ Texte | le focus est gardé pendant la frappe | bon |
| Dialogue de confirmation | ouvert, focus sur le bouton primaire ; `focus()` sur Enregistrer, hors du dialogue, ne sort pas (le reste est inerte) ; à la fermeture, le focus revient au bouton déclencheur | bon |
| Dialogue d'import | focus sur **Annuler** quand la barre lance des commandes ; reste inerte ; focus rendu à `#importer` | très bon |

Quand le focus tombe sur `body`, VoiceOver repart du haut de la page : Inès doit refaire toute la tabulation pour revenir à l'endroit où elle était. Cela arrive trois fois dans le parcours principal : choisir un élément, choisir un symbole, supprimer. Cause commune : `selectionner()` et `dessinerProprietes(true)` remplacent le HTML (`innerHTML`, l. 808 et 1060). Le focus n'est rendu que s'il était sur un élément qui a un `id` (l. 1013 et 1062) ; or les éléments de l'aperçu et les boutons de symboles n'en ont pas.

Le glisser-déposer a des équivalents clavier : ⌥←/→ ou ◀ ▶ pour réordonner, « Choisir une image… » à la place du dépôt d'une image, « Importer… » à la place du dépôt d'un `.toml` ou d'un `.zip`. Mais le sous-titre ne parle que de glisser (l. 331 : « glisse-le pour le déplacer »), et le raccourci ⌥←/→ n'est annoncé nulle part.

## Problèmes par critère WCAG

| # | Critère | Preuve | Gravité | Correctif proposé |
|---|---|---|---|---|
| 1 | 2.4.3 Ordre du focus / 2.1.1 Clavier | Entrée sur `#barre .el`, clic sur `.grille button`, `#p-supprimer` : `activeElement` = `body` (l. 729, 808, 1060-1062, 1098, 1103) | **Bloquant** | Après chaque redessin, rendre le focus à l'élément équivalent : `.el[data-i=sel.i]` pour l'aperçu, `[data-symbole=…]` pour la grille ; après une suppression, l'élément voisin ou, à défaut, `#barre` |
| 2 | 4.1.2 Nom, rôle, valeur | Les options de l'aperçu s'appellent seulement « Bouton », « Bouton », « Bouton », « Curseur », « Curseur » (`aria-label` l. 808 : type + `texte` seulement). Le symbole et l'action ne sont pas dits | **Majeur** | Nom = type + texte, symbole ou app + action, par exemple « Bouton, apple.logo, afficher la barre d'Apple, 1 sur 9 » |
| 3 | 4.1.2 | `aria-selected="null"` sur toutes les options tant que rien n'est choisi (l. 808 : `sel && …` donne `null`) ; pas d'`aria-orientation="horizontal"` sur `#barre` (l. 286), alors que la navigation se fait avec ←/→ | Moyen | `aria-selected="${!!(sel && sel.i === i)}"` et `aria-orientation="horizontal"` |
| 4 | 4.1.2 | Grille de symboles : `role="listbox"` > `button role="option"` avec `aria-pressed` (l. 982). `aria-pressed` n'est pas valide sur une option, donc le symbole choisi n'est pas annoncé. Les 48 options sont toutes dans l'ordre de tabulation (`tabIndex` 0) | **Majeur** | Soit une grille de vrais boutons sans rôle (`aria-pressed` est alors correct), soit une listbox avec `aria-selected` et un tabindex itinérant manipulé aux flèches, comme l'aperçu |
| 5 | 3.3.4 Prévention des erreurs / 2.1.1 | Retour arrière ou Suppr sur un élément de l'aperçu supprime aussitôt (l. 868), sans annulation ; un appui de trop efface un groupe entier | Moyen | Annuler (⌘Z) ou un message « Bouton supprimé · Annuler » dans `#etat`, avec un bouton focusable |
| 6 | 3.3.2 Étiquettes ou instructions | Le déplacement au clavier (⌥←/→, Suppr) n'est indiqué nulle part ; le sous-titre ne parle que de glisser (l. 331) | Moyen | Ajouter au sous-titre : « Au clavier : flèches pour choisir, ⌥ + flèches pour déplacer » ; `aria-describedby` sur `#barre` |
| 7 | 1.4.11 Contraste non textuel / 2.4.7 | Anneau de focus `--anneau` à 35 % d'opacité (l. 20, 37, 56) : **1,69:1** sur la fenêtre et 1,72:1 sur les encarts en clair, **2,11:1** et 2,00:1 en sombre. Il faut 3:1. Voir `captures/ines-anneau-focus-*.png` : en clair, on le devine à peine | **Majeur** (Inès est malvoyante) | Anneau opaque `outline: 2px solid var(--accent); outline-offset: 2px`, soit #0063e1 (5,4:1) en clair et #3b9bff en sombre. Les champs texte sont déjà corrects, grâce à leur bordure accent au focus (l. 76) |
| 8 | 1.4.11 | Interrupteur éteint : rail `--rail` **1,17:1** sur l'encart en clair, 1,28:1 en sombre ; le bouton blanc se fond dans l'encart blanc (`captures/ines-interrupteur-eteint-clair.png`) | Majeur | Rail éteint avec une bordure d'au moins 3:1, par exemple `box-shadow: inset 0 0 0 1px #8e8e93` en clair |
| 9 | 1.4.11 | Bordure des champs `--champ-bord` : 1,45:1 en clair, 1,56:1 en sombre ; segment FR/EN choisi contre non choisi : 1,26:1 en clair, 1,92:1 en sombre (seule la graisse 500 les distingue) | Moyen | Bordure de champ à 0,45 d'opacité ou plus ; segment choisi avec une bordure ou un fond accent |
| 10 | 1.4.3 Contraste du texte | Placeholder `--texte3` : **3,26:1** en clair (#8e8e93 sur #fff) et 3,45:1 en sombre. Le placeholder « automatique » des champs Taille et Largeur porte une information, la valeur par défaut (l. 1005) | Moyen | `--texte3` à #6e6e73 en clair (≥ 4,5:1) et #8e8e93 en sombre, ou dire « automatique » dans la légende |
| 11 | 1.4.3 | Lien accent en sombre (#3b9bff sur l'encart #323234) : **4,46:1**, ce qui touche « Utiliser … », « Retour au groupe », « Choisir une autre image » | Faible | #4da6ff ou plus clair |
| 12 | 1.4.4 Redimensionnement du texte | Toutes les tailles sont en px (`body` 13px, l. 46) : passer la police racine à 200 % ne change rien (13px → 13px). La WKWebView n'a ni `allowsMagnification` ni ⌘+/⌘−, et la fenêtre n'a pas de menu (`mod.rs:668-695`) : seul le Zoom de macOS agrandit | **Majeur** | ⌘+ / ⌘− / ⌘0 qui règlent `document.documentElement.style.zoom` (mémorisé), ou `allowsMagnification` ; tailles en rem |
| 13 | 1.4.4 / 1.4.10 Redistribution | Texte doublé : la page fait 1339 px pour une fenêtre de 1180. **Enregistrer sort de l'écran**, « Supprimer » et « Aucun symbole » sont coupés, les libellés de la palette tronqués (« Musique en cou », « Résultat d'une c »), voir `captures/ines-texte-200-clair.png`. Avec `zoom: 2`, la largeur passe à 2124 px et il faut défiler dans les deux sens, à cause de `min-width: 1060px` (l. 48) | Majeur | `header` en `flex-wrap: wrap` ; palette en `minmax(228px, auto)` ; en dessous d'environ 900 px, aperçu à l'échelle (`transform: scale`) et retrait du `min-width` |
| 14 | 4.1.3 Messages d'état | `#proprietes` porte `aria-live="polite"` (l. 301) et il est entièrement reconstruit à chaque choix, à chaque symbole, et même à chaque lettre tapée dans « Nom exact » (l. 1079) : VoiceOver relit tout le panneau. `#etat` répète « Modifications non enregistrées. » à chaque changement (l. 683) | Moyen | Retirer `aria-live` de `#proprietes` (le focus suffit) ; dans `etat()`, ne rien réécrire quand le texte n'a pas changé ; passer les erreurs dans une région `role="alert"` à part |
| 15 | 2.4.6 Titres et étiquettes | Interrupteurs de couleur nommés seulement « gris de la barre » et « blanc » (l. 1002) ; tous les boutons du contenu d'un groupe s'appellent « Régler » (l. 1056) ; `aria-label="Nom exact"` remplace le libellé « Symbole » (l. 978) | Moyen | « Couleur automatique (gris de la barre) » ; `aria-label="Régler Bouton · Pomme"` ; `aria-label="Symbole, nom exact"` |
| 16 | 3.3.4 | Dialogue « Remplacer ta barre par “Simple” ? » : le focus va au bouton primaire, **Remplacer** (l. 704). Un Entrée réflexe efface les modifications | Moyen | Pour une action destructive, focus sur Annuler, comme le fait déjà l'import (l. 630) |
| 17 | 1.1.1 / 1.3.1 | Dialogue d'import : l'aperçu de la barre reçue est `aria-hidden` (l. 313). Pour VoiceOver, il ne reste que « 2 éléments » | Faible | Une liste masquée visuellement, de la forme « Bouton “Build”, Heure » |
| — | 1.4.3, bouton désactivé (exempté) | « Revenir à la version précédente » désactivé : 2,84:1 en clair, 2,86:1 en sombre (opacité 0,45, l. 64). Exempté par la WCAG, mais illisible pour Inès | Info | Laisser le libellé lisible et dire pourquoi (« aucune version précédente ») |
| — | Non vérifié | Sans menu Édition dans `--personnaliser`, ⌘C, ⌘V, ⌘Z et ⌘A dans les champs ne marchent peut-être pas : à tester à la main | À vérifier | Menu principal minimal avec Édition (Couper, Copier, Coller, Tout sélectionner, Annuler) |

Les autres textes passent AA dans les deux thèmes (mesures de `contrastes` dans le journal du pilote) :

| Texte | Clair | Sombre |
|---|---|---|
| `.aide` | 5,54 | 5,73 |
| h2 de la palette (11 px) | 5,03 | 6,25 |
| `.legende` (11 px) | 5,99 | 4,97 |
| Control Strip | 5,36 | 5,36 |
| Bandeau d'erreur, texte | 14,74 | 11,92 |
| Bandeau d'erreur, icône | 5,47 | 5,17 |
| `--erreur` sur encart | 6,24 | 4,54 (juste) |
| `--ok` | 5,0 | 7,29 |
| Textes des dialogues | ≥ 5,4 | ≥ 5,4 |

## Ce qui est bien fait

- `<html lang>` suit la langue choisie (l. 1163) ; boutons et champs ont tous un nom (aucun contrôle sans nom trouvé) ; boutons icônes ◀ ▶ avec `aria-label` (l. 1016) ; tuiles et vignettes décoratives en `aria-hidden`.
- Aperçu en tabindex itinérant : un seul arrêt de tabulation pour toute la barre, puis les flèches. Réordonner au clavier existe (⌥←/→, ◀ ▶) et le focus suit l'élément déplacé.
- Interrupteurs : de vraies `input type=checkbox role=switch` avec `<label for>`, donc l'état coché est exposé nativement.
- `<dialog>` modaux natifs : le fond est inerte, Échap vaut Annuler, le focus revient au déclencheur. Pour l'import, le focus va à Annuler quand la barre lance des commandes, ce qui est exactement le bon réflexe.
- Messages d'état dans `#etat` (`role="status"`), avec des phrases complètes et une erreur qui dit quoi faire ; le texte du bandeau d'erreur est à 14,7:1.
- `prefers-reduced-motion` : les transitions n'existent que sous `no-preference` (l. 260-262) et il n'y a aucune `@keyframes` (0 animation mesurée).
- Cibles : aucune cible interactive sous 24 × 24 px qui ne soit pas couverte par son libellé ou par l'espacement (2.5.8). Interrupteurs 34 × 20 px, mais leur libellé est cliquable et ils sont seuls sur une ligne de 44 px ; grille de symboles 36 px ; liens de 22 px seulement dans une phrase (exception « en ligne »).
- Champs texte : bordure accent opaque au focus (5,4:1 en clair).

## Priorités

1. **Focus rendu après chaque redessin** (#1) : sans ça, Inès ne peut ni choisir un élément, ni choisir un symbole, ni supprimer sans repartir du haut de la page. C'est un correctif de quelques lignes, dans `selectionner`, dans le gestionnaire de la grille et dans `supprimer`.
2. **Anneau de focus opaque** (#7) et **interrupteur éteint visible** (#8) : deux jetons CSS, et tout le parcours devient lisible au Zoom.
3. **Noms parlants dans l'aperçu et état de la grille de symboles** (#2, #4) : aujourd'hui, VoiceOver annonce « Bouton, Bouton, Bouton ».
4. **Agrandissement dans l'app et mise en page qui tient à 200 %** (#12, #13) : ⌘+ et ⌘−, `header` qui passe à la ligne.
5. Le reste par lot : `aria-live` trop bavard (#14), suppression annulable (#5), aide clavier (#6), placeholders (#10), focus sur Annuler pour « Remplacer » (#16), libellés ambigus (#15).
