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

# Pour les GIF : un pointeur dessiné dans la page (les captures WebKit n'ont pas celui du système) et des images
# numérotées `NNNN-hK.png`, K = nombre de fois que l'image est répétée (les temps d'arrêt).
animation_js() {
    cat <<'JS'
let numero = 0;
const image = async (repeter = 1) => { capture(String(++numero).padStart(4, '0') + '-h' + repeter + '.png'); await attendre(140); };
const pointeur = document.createElement('div');
pointeur.innerHTML = '<svg width="22" height="30" viewBox="0 0 22 30"><path d="M2 2 L2 24 L7.5 18.5 L11.5 27.5 L15 26 L11 17.2 L19 17.2 Z" fill="#000" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>';
Object.assign(pointeur.style, { position: 'fixed', left: '0', top: '0', zIndex: 99999, pointerEvents: 'none', filter: 'drop-shadow(0 1px 2px rgb(0 0 0 / 0.4))' });
document.body.append(pointeur);
let px = 640, py = 420;
const placer = (x, y) => { px = x; py = y; pointeur.style.transform = `translate(${x - 2}px, ${y - 2}px)`; };
placer(px, py);
const centre = el => { const r = el.getBoundingClientRect(); return [r.left + Math.min(r.width / 2, 40), r.top + r.height / 2]; };
// Glisse vers l'élément en `n` images, avec une courbe douce (ease-in-out).
const aller = async (el, n = 8) => {
  const [x0, y0] = [px, py], [x1, y1] = centre(el);
  for (let k = 1; k <= n; k++) { const t = k / n, e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2; placer(x0 + (x1 - x0) * e, y0 + (y1 - y0) * e); await image(); }
};
const cliquer = async (el, n = 8) => { await aller(el, n); el.click(); document.activeElement?.blur(); await image(3); };
// Le champ est relu à chaque lettre : certaines saisies redessinent le panneau (recherche de symbole).
const taper = async (selecteur, texte) => {
  let deja = q(selecteur).value;
  for (const c of texte) { const el = q(selecteur); deja += c; el.value = deja; el.dispatchEvent(new Event('input', { bubbles: true })); await image(); }
  await attendre(400); await image(3);
};
const q = s => document.querySelector(s);
JS
}
