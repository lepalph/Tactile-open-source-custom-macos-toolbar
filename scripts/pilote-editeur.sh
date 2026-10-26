# Sourcé par scripts/hero.sh et scripts/gifs.sh : ouvre l'éditeur sur une barre, le fait piloter par un script JS, attend la fin.
# piloter <barre.toml> <pilote.js> <dossier> : le pilote finit en capturant <dossier>/fin.png ; l'éditeur est alors arrêté.
# Seule la fenêtre de réglage tourne (--personnaliser) : la Touch Bar de l'utilisateur n'est pas touchée, ni ~/.config/tactile.
piloter() {
    local bin=${TACTILE_BIN:-target/release/tactile}
    [[ -x $bin ]] || mise exec -- cargo build --release --quiet
    rm -f "$3/fin.png"
    TACTILE_CONFIG=$1 TACTILE_APPARENCE=sombre TACTILE_EDITEUR_JS=$2 $bin --personnaliser >/dev/null 2>&1 &
    local pid=$!
    for _ in {1..900}; do [[ -s $3/fin.png ]] && break; sleep 0.1; done
    kill $pid 2>/dev/null; wait $pid 2>/dev/null || true
    [[ -s $3/fin.png ]] || { echo "l'éditeur n'a pas fini ses captures" >&2; return 1; }
}

# Le début commun des pilotes : capture, attente, langue anglaise (la langue choisie avant est rendue par `finir`).
# L'éditeur partage son localStorage avec celui de l'utilisateur (base file:///).
prelude_js() {
    cat <<JS
const DOSSIER = '$1';
const attendre = ms => new Promise(r => setTimeout(r, ms));
const capture = nom => window.webkit.messageHandlers.tactile.postMessage(JSON.stringify({ type: 'capture', chemin: DOSSIER + '/' + nom }));
let langueAvant = null; try { langueAvant = localStorage.getItem('langue'); } catch (_) {}
// Écran verrouillé, WebKit ne joue pas les transitions CSS : sans ça, une capture garde l'état d'avant (FR encore choisi).
const sansTransition = document.createElement('style');
sansTransition.textContent = '*, *::before, *::after { transition: none !important; }';
document.head.append(sansTransition);
document.querySelector('[data-langue="en"]').click();
const finir = async () => {
  try { langueAvant === null ? localStorage.removeItem('langue') : localStorage.setItem('langue', langueAvant); } catch (_) {}
  await attendre(600); capture('fin.png');
};
JS
}
