#!/bin/zsh
# Les GIF du README, en anglais et en thème sombre, rendus par l'éditeur piloté sur exemples/vitrine.toml :
#   docs/demo-editeur.gif : un symbole pour le tiroir, un lanceur ajouté dedans, Save ;
#   docs/demo-logo.gif    : un logo posé sur un bouton, puis Monochrome.
# Chaque image est une capture WebKit (marche écran verrouillé) ; gifski les assemble (brew install gifski).
# Pas de GIF de la vraie Touch Bar : `screencapture -b` ne la voit pas écran verrouillé, ni sans présentation par une app.
set -euo pipefail
cd "${0:A:h}/.."
source scripts/pilote-editeur.sh
command -v gifski >/dev/null || { echo "gifski manque : brew install gifski" >&2; exit 1; }

# Le logo copié par l'éditeur s'affiche avec son chemin : un dossier lisible sous le home plutôt que /var/folders.
dossier=$HOME/Library/Caches/tactile-demo
rm -rf "$dossier"; mkdir -p "$dossier"
trap 'rm -rf "$dossier"' EXIT

# assembler <nom.gif> <largeur> <recadrage sips "h l y x" ou vide>
assembler() {
    local sortie=$1 largeur=$2 rognage=$3 suite=$dossier/suite n=0
    rm -rf "$suite"; mkdir -p "$suite"
    for f in "$dossier"/[0-9][0-9][0-9][0-9]-h*.png; do
        local r=${${f:t:r}#*-h}
        [[ -n $rognage ]] && sips -c ${=rognage[(w)1,2]} --cropOffset ${=rognage[(w)3,4]} "$f" >/dev/null
        for _ in {1..$r}; do (( ++n )); ln -s "$f" "$suite/$(printf %05d $n).png"; done
    done
    gifski --quiet --fps 12 --quality 82 --width $largeur -o "$sortie" "$suite"/*.png
    [[ -z ${QA:-} ]] || { mkdir -p "$QA/${sortie:t:r}"; cp "$dossier"/[0-9][0-9][0-9][0-9]-h*.png "$QA/${sortie:t:r}/"; }
    rm -f "$dossier"/[0-9][0-9][0-9][0-9]-h*.png
}

# --- 1. Le tiroir : symbole « tray.fill », un lanceur Mail ajouté, Save.
cp exemples/vitrine.toml "$dossier/barre.toml"
{ prelude_js "$dossier"; animation_js; cat <<'JS'
(async () => {
  await attendre(2500);
  await image(6);
  await cliquer(q('#barre .el[data-i="1"]'), 10);
  await cliquer(q('#symbole-recherche'));
  await taper('#symbole-recherche', 'tray');
  await cliquer(q('.grille [data-symbole="tray.fill"]'));
  await image(8);
  const ajout = q('#f-ajout-groupe');
  await aller(ajout); ajout.value = 'lanceur'; ajout.dispatchEvent(new Event('change', { bubbles: true })); await image(4);
  const app = q('#f-app');
  await aller(app); app.value = ''; app.dispatchEvent(new Event('input', { bubbles: true })); await image(2);
  await taper('#f-app', 'Mail');
  await attendre(600); await image(6);
  await cliquer(q('#p-retour'));
  await image(10);
  await cliquer(q('#enregistrer'), 10);
  await attendre(800); await image(20);
  await finir();
})();
JS
} > "$dossier/pilote.js"
piloter "$dossier/barre.toml" "$dossier/pilote.js" "$dossier"
assembler docs/demo-editeur.gif 960 "${RECADRAGE_EDITEUR:-}"

# --- 2. Un logo sur un bouton : Button, texte « Deploy », le logo, Monochrome.
cp exemples/vitrine.toml "$dossier/barre.toml"
cat > "$dossier/deploy.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">
  <path fill="#FF7A1A" fill-rule="evenodd" d="M32 2a30 30 0 1 1 0 60a30 30 0 1 1 0-60z M32 14 L16 32 L26 32 L26 48 L38 48 L38 32 L48 32 Z"/>
</svg>
SVG
{ prelude_js "$dossier"; animation_js; cat <<JS
(async () => {
  await attendre(2500);
  await image(6);
  await cliquer(q('#ajouts [data-ajout="bouton"]'), 10);
  const texte = q('#f-texte');
  await cliquer(texte); await taper('#f-texte', 'Deploy');
  // « Choose an image… » ouvrirait un sélecteur de fichiers : le pointeur y va, le pilote importe le fichier lui-même.
  await aller(q('#image-choisir'), 10); await image(3);
  window.webkit.messageHandlers.tactile.postMessage(JSON.stringify({ type: 'importerImage', chemin: '$dossier/deploy.svg' }));
  await attendre(1500); await image(12);
  await cliquer(q('#f-gabarit'), 10);
  await image(16);
  await finir();
})();
JS
} > "$dossier/pilote.js"
piloter "$dossier/barre.toml" "$dossier/pilote.js" "$dossier"
assembler docs/demo-logo.gif 960 "${RECADRAGE_LOGO:-}"

ls -l docs/demo-editeur.gif docs/demo-logo.gif
