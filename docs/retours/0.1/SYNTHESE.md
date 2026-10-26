# Synthèse des retours sur la 0.1

Quatre personas (rapports dans ce dossier) : Nadia (débutante), Marc (avancé, sur la vraie barre), Lise (BetterTouchTool), Sam (dev anglophone). Carnet ordonné ; chaque item dit la preuve qui le ferme.

## A. Bugs

| # | Bug | Source | Gravité | Preuve de correction |
|---|---|---|---|---|
| A1 | Symbole SF inexistant → panique au rechargement, Touch Bar noire, replante au relancement | Marc | bloquant | fait : tests `symbole_inconnu_ne_donne_pas_d_image`, `symbole_inconnu_donne_un_bouton_texte` ; journal « symbole SF inconnu « x » (élément n) » |
| A2 | Gatekeeper Sequoia : « clic droit → Ouvrir » ne marche plus pour une app ad hoc | Nadia | bloquant | README : chemin Réglages → Confidentialité et sécurité → « Ouvrir quand même », et commande `xattr -d com.apple.quarantine` en alternative ; icône d'app |
| A3 | Erreur de config invisible (journal caché), barre « qui ne marche plus » | Nadia, Marc | majeur | fait : élément rouge « ⚠ config ligne N » dans la barre (les notifications macOS exigent un bundle), autotest étapes 20-21 + écran « erreur » de verifier.sh ; menu « Voir le journal » ; `tactile --verifier` (test `verifier_dit_la_ligne_ou_l_absence_du_fichier`) |
| A4 | Éléments qui débordent disparaissent sans message ; texte long avale la barre | Marc | majeur | fait : 200 pt max pour bouton texte / heure / en_cours sans `largeur` (capture) ; journal « la barre déborde … n élément(s) seront cachés », test `debordement_compte_les_elements_caches` (estimation à un élément près) |
| A5 | Scripts : stderr et code retour jetés ; script sans fin → orphelins à chaque rechargement | Marc | majeur | fait : tests `un_script_trop_long_est_tue_avec_son_groupe` (`sleep 30`), `stderr_et_code_de_retour_remontent`, `une_barre_remplacee_tue_ses_scripts` ; vérifié sur la barre (jamais plus d'un `sleep 1000` vivant). Reste : quitter Tactile pendant un script le laisse finir seul |
| A6 | `masquer_control_strip` : doc dit `true` par défaut, code dit `false` ; sans effet à chaud | Nadia, Marc | majeur | fait : test `control_strip_masque_par_defaut` ; à chaud, la barre est minimisée puis représentée avec le nouveau placement (retour vers `true` vu en capture). Preuve visuelle du placement Control Strip reportée : sans toucher humain la Touch Bar reste noire en placement 0, pas d'étape d'autotest possible ; à vérifier au doigt |
| A7 | `cargo doc` : lien intra-doc cassé `[[element]]` config.rs:83 | Sam | mineur | fait : `cargo doc --no-deps` sans avertissement |
| A8 | `main.rs` ignore argv : `tactile --help` lance l'app | Sam | mineur | fait : `--help` (10 lignes), `--version`, `--verifier`, autre argument → code 2 ; tests `chaque_argument`, `aide_en_dix_lignes` |

## B. Ergonomie

| # | Point | Source | État |
|---|---|---|---|
| B1 | Pas d'éditeur visuel : tout passe par un fichier caché et un vocabulaire (TOML, SF Symbols, hex) inconnu d'une débutante | Nadia | chantier 3b |
| B2 | Lancer une app = `shell = "open -a Nom"` avec le nom anglais, jamais expliqué | Nadia, Lise | fait : `{ app = "Nom" }`, README « Trouver le nom d'une app », app introuvable au journal |
| B3 | Anglais : 19 lignes en pied de README, erreurs traduites en français, CONTRIBUTING impose le français | Sam | chantier 3c |
| B4 | Clés TOML françaises : frein international → alias anglais `#[serde(alias)]`, glossaire testé | Sam, Lise | fait : alias sur chaque clé, type, réglage, action ; `exemples/barre.en.toml` ; tests `alias_anglais_donnent_la_meme_barre`, `glossaire_du_readme` |
| B5 | Présentation GitHub : pas de bannière, badges, GIF, notes de release, gabarits d'issues, icône | Nadia, Sam | chantier 3c |

## C. Personnalisation (par valeur)

| # | Fonction | Syntaxe souhaitée | Source | Preuve |
|---|---|---|---|---|
| C1 | Raccourci clavier comme action | `action = { touche = "cmd+shift+4" }` | Marc, Lise | fait : modificateurs cmd/shift/ctrl/alt/fn, touches nommées, caractères résolus sur la disposition (AZERTY), `{ keycode = n }` ; tests `raccourci_lu_et_resolu`, `touche_inconnue_dit_la_ligne` ; autotest 23 (f15) |
| C2 | Lancer une app / ouvrir une URL | `action = { app = "Figma" }`, `{ url = "https://…" }` | Nadia, Lise | fait : `open -a` / `open`, stderr au journal ; test `app_introuvable_et_applescript_faux_remontent_leur_erreur` ; autotest 24 (TextEdit ouvert) |
| C3 | Barre par application | `[[application]] app = "com.adobe.PremierePro"` + ses `[[application.element]]`, `herite = true` | Lise, Marc | chantier 3a-3 |
| C4 | Apparence : image perso, couleur du texte, taille, icône + texte | `image = "~/x.png"`, `couleur_texte`, `taille`, `affichage = "icone"\|"texte"\|"les_deux"` | Lise, Marc | fait : tests `apparence_d_un_bouton`, `image_redimensionnee_ou_absente`, `image_introuvable_donne_un_bouton_texte` ; autotest 23 et capture « riche » |
| C5 | `largeur` sur `groupe` (libellé tronqué) | `largeur = 100` | Marc | fait : test `groupe_avec_largeur_et_batterie` ; « Export › » à 140 pt dans la capture « riche » |
| C6 | Scripts riches : sortie JSON `{"texte","symbole","couleur"}` | automatique si la sortie commence par `{` | Lise | fait : clés texte, symbole, image, couleur, couleur_texte ; test `sortie_json_ou_texte_brut` ; autotest 23 (« ok » vert) |
| C7 | Widget batterie | `type = "batterie"` | Marc, Lise | fait : `pmset -g batt` toutes les 30 s, `symbole`, `largeur` ; tests `analyse_pmset`, `chaque_symbole_de_pile_existe` ; autotest 23 |
| C8 | AppleScript natif | `action = { applescript = "…" }` | Lise | fait : `osascript -e`, stderr au journal ; autotest 25 (TextEdit fermé) |
| C9 | Import de presets BTT | `tactile --importer x.bttpreset` | Lise | reporté après la 0.2 |

## Ce que Tactile fait déjà mieux (à garder)

MIT, 14 Mo et 0,1 % CPU au repos, config texte rechargée à chaud, erreurs avec numéro de ligne, groupes, reprenabilité documentée.

## Découpage phase 3

- **3a-1 Solidité** : A1, A3, A4, A5, A6, A7, A8 (même fichiers, un agent, sur la vraie barre).
- **3a-2 Personnalisation** : C1, C2, C4, C5, C6, C7, C8, B4 (alias anglais), B2 (doc).
- **3a-3 Barre par application** : C3.
- **3b Éditeur visuel** : B1 (fenêtre « Personnaliser ma barre »).
- **3c Open source** : A2, B3, B5, README anglais + README.fr.md.
- Reporté : C9 (import BTT, après la 0.2).
