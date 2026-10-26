#!/bin/zsh
# Les visuels du dépôt, refaits depuis exemples/vitrine.toml rendue par l'éditeur (en anglais, thème sombre) :
#   docs/hero.png            (1600 px de large) : la barre au-dessus de la fenêtre de l'éditeur ;
#   docs/social-preview.png  (1280×640) : l'image d'aperçu GitHub, à déposer à la main dans Settings ;
#   docs/editeur-erreur-sombre-en.png : un symbole mal écrit signalé, pour le README anglais (le français garde la sienne).
# La bande « Touch Bar » est l'aperçu de l'éditeur recadré : il marche écran verrouillé, `screencapture -b` non.
# Valeurs réelles au moment de la capture : météo de Paris (Open-Meteo), batterie (pmset), heure. Le morceau en
# cours est un exemple fixe (l'éditeur n'affiche pas la musique). Outils : swift ; pngquant s'il est là.
set -euo pipefail
cd "${0:A:h}/.."
source scripts/pilote-editeur.sh

dossier=$(mktemp -d)
trap 'rm -rf "$dossier"' EXIT
cp exemples/vitrine.toml "$dossier/barre.toml"
meteo=$(curl -s --max-time 10 'https://api.open-meteo.com/v1/forecast?latitude=48.85&longitude=2.35&current=temperature_2m,weather_code' || true)
[[ $meteo == \{* ]] || meteo=null
pct=$(pmset -g batt | grep -o '[0-9]*%' | head -1 | tr -d %); pct=${pct:-100}
if pmset -g batt | grep -q 'AC Power'; then pile=battery.100percent.bolt
elif (( pct >= 88 )); then pile=battery.100percent; elif (( pct >= 63 )); then pile=battery.75percent
elif (( pct >= 38 )); then pile=battery.50percent; else pile=battery.25percent; fi

{ prelude_js "$dossier"; cat <<JS
(async () => {
  // La météo comme la barre l'affiche : le gabarit rempli par la vraie réponse, le symbole choisi par symbole_selon.
  const meteo = $meteo;
  const e = barre.element.find(x => x.type === 'texte');
  if (meteo && e) {
    const lire = p => p.split('.').reduce((o, c) => o == null ? o : o[c], meteo);
    e.gabarit = e.gabarit.replace(/\{([^}]*)\}/g, (_, p) => lire(p.trim()) ?? '—');
    e.symbole_selon = { ...e.symbole_selon, defaut: e.symbole_selon.valeurs[String(lire(e.symbole_selon.champ))] || e.symbole_selon.defaut };
  }
  demanderGlyphes(['$pile', ...symbolesDe(barre.element)]);
  const dessiner = dessinerBarre;
  dessinerBarre = () => {
    dessiner();
    document.querySelectorAll('#barre [data-type=en_cours] .face').forEach(f => { f.innerHTML = '▶&nbsp;&nbsp;Clair de Lune&nbsp;&nbsp;·&nbsp;&nbsp;Claude Debussy'; });
    document.querySelectorAll('#barre [data-type=batterie] .face').forEach(f => { f.innerHTML = sf('$pile') + '<span>$pct %</span>'; });
  };
  await attendre(2500);
  // La bande : la barre seule, sans sélection ni repères des espaces souples.
  const sansRepere = document.createElement('style');
  sansRepere.textContent = '.el[data-type="souple"] .face::before { visibility: hidden; }';
  document.head.append(sansRepere);
  sel = null; tout(); document.activeElement?.blur();
  await attendre(800); capture('barre.png');
  await attendre(1200); sansRepere.remove();
  // L'éditeur : le tiroir choisi, ses quatre lanceurs avec leur icône. Une recherche courte replie la grille de
  // symboles pour que la liste tienne dans la fenêtre.
  rechercheSymbole = 'sun'; selectionner({ i: 1 }); document.activeElement?.blur();
  await attendre(1500); capture('editeur.png');
  await attendre(1200); await finir();
})();
JS
} > "$dossier/pilote.js"
piloter "$dossier/barre.toml" "$dossier/pilote.js" "$dossier"

swift - "$dossier/barre.png" "$dossier/editeur.png" assets/icone-1024.png docs/hero.png docs/social-preview.png <<'SWIFT'
import AppKit

let a = CommandLine.arguments
func charger(_ chemin: String) -> CGImage {
    let src = CGImageSourceCreateWithURL(URL(fileURLWithPath: chemin) as CFURL, nil)!
    return CGImageSourceCreateImageAtIndex(src, 0, nil)!
}
let icone = charger(a[3])
// La barre : le rectangle noir de l'aperçu, trouvé dans la capture (les lignes et colonnes en majorité noires).
let barre: CGImage = {
    let page = charger(a[1])
    let (l, h) = (page.width, page.height)
    let ctx = CGContext(data: nil, width: l, height: h, bitsPerComponent: 8, bytesPerRow: l * 4,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.draw(page, in: CGRect(x: 0, y: 0, width: l, height: h))
    let px = ctx.data!.bindMemory(to: UInt8.self, capacity: l * h * 4)
    func noir(_ x: Int, _ y: Int) -> Bool { let i = (y * l + x) * 4; return px[i] < 6 && px[i + 1] < 6 && px[i + 2] < 6 }
    let lignes = (0..<h).filter { y in stride(from: 0, to: l, by: 2).filter { noir($0, y) }.count > l / 5 }
    let (haut, bas) = (lignes.first!, lignes.last!)
    let milieu = (haut + bas) / 2
    let cols = (0..<l).filter { noir($0, milieu) }
    // Les coins arrondis laissent un peu du fond de la page dans les angles : `dessinerBarre` les recoupe.
    return page.cropping(to: CGRect(x: cols.first!, y: haut, width: cols.last! - cols.first! + 1, height: bas - haut + 1))!
}()
let editeur = charger(a[2])

func couleur(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
}
func arrondi(_ r: CGRect, _ rayon: CGFloat) -> CGPath { CGPath(roundedRect: r, cornerWidth: rayon, cornerHeight: rayon, transform: nil) }
func toile(_ l: Int, _ h: Int) -> CGContext {
    let ctx = CGContext(data: nil, width: l, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .high
    // Fond : anthracite, à peine plus clair en haut (le dessus du clavier, pas un effet).
    let d = CGGradient(colorsSpace: ctx.colorSpace, colors: [couleur(0x232326), couleur(0x161618)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(d, start: CGPoint(x: 0, y: CGFloat(h)), end: .zero, options: [])
    return ctx
}
// La Touch Bar dans son cadre noir arrondi, comme sur la machine.
func dessinerBarre(_ ctx: CGContext, _ r: CGRect) {
    let pad = r.height * 0.12
    ctx.addPath(arrondi(r, r.height * 0.22)); ctx.setFillColor(couleur(0x000000)); ctx.fillPath()
    ctx.addPath(arrondi(r.insetBy(dx: 0.5, dy: 0.5), r.height * 0.22)); ctx.setStrokeColor(couleur(0xFFFFFF, 0.10)); ctx.setLineWidth(1); ctx.strokePath()
    let l = r.width - 3 * pad, h = l * CGFloat(barre.height) / CGFloat(barre.width)
    let dedans = CGRect(x: r.midX - l / 2, y: r.midY - h / 2, width: l, height: h)
    ctx.saveGState(); ctx.addPath(arrondi(dedans, h * 0.2)); ctx.clip(); ctx.draw(barre, in: dedans); ctx.restoreGState()
}
func ecrire(_ l: Int, _ h: Int, _ ctx: CGContext, _ chemin: String) {
    let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: chemin) as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(dest, ctx.makeImage()!, nil); CGImageDestinationFinalize(dest)
}

// --- Hero : 1600 de large, marge 64, la barre puis l'éditeur (ratio des captures conservé).
do {
    let L: CGFloat = 1600, m: CGFloat = 64, utile = L - 2 * m
    let hBarre = utile * CGFloat(barre.height) / CGFloat(barre.width) / 0.76  // la capture occupe 76 % de la hauteur du cadre
    let hEd = utile * CGFloat(editeur.height) / CGFloat(editeur.width)
    let H = m + hBarre + 40 + hEd + m
    let ctx = toile(Int(L), Int(H.rounded()))
    let yEd = m
    let cadre = CGRect(x: m, y: yEd, width: utile, height: hEd), fenetre = arrondi(cadre, 18)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 48, color: couleur(0x000000, 0.55))
    ctx.addPath(fenetre); ctx.setFillColor(couleur(0x262628)); ctx.fillPath()
    ctx.restoreGState()
    ctx.saveGState(); ctx.addPath(fenetre); ctx.clip(); ctx.draw(editeur, in: cadre); ctx.restoreGState()
    dessinerBarre(ctx, CGRect(x: m, y: yEd + hEd + 40, width: utile, height: hBarre))
    ecrire(Int(L), Int(H), ctx, a[4])
}

// --- Aperçu social : 1280×640, icône + nom + phrase, la barre en bas.
do {
    let (L, H): (CGFloat, CGFloat) = (1280, 640)
    let ctx = toile(Int(L), Int(H))
    ctx.draw(icone, in: CGRect(x: 72, y: 330, width: 200, height: 200))
    func texte(_ s: String, _ taille: CGFloat, _ poids: NSFont.Weight, _ c: CGColor, _ x: CGFloat, _ y: CGFloat) {
        let attr: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: taille, weight: poids), .foregroundColor: c]
        let ligne = CTLineCreateWithAttributedString(NSAttributedString(string: s, attributes: attr))
        ctx.textPosition = CGPoint(x: x, y: y); CTLineDraw(ligne, ctx)
    }
    texte("Tactile", 96, .bold, couleur(0xF2F2F4), 300, 432)
    texte("A free, open-source Touch Bar for your MacBook Pro,", 34, .regular, couleur(0xB4B4BA), 304, 380)
    texte("written in a text file or set up in a window.", 34, .regular, couleur(0xB4B4BA), 304, 338)
    texte("Native Rust · MIT · github.com/lepalph/tactile-touch-bar", 24, .medium, couleur(0x8A8A90), 304, 286)
    let lb = L - 144
    dessinerBarre(ctx, CGRect(x: 72, y: 96, width: lb, height: lb * CGFloat(barre.height) / CGFloat(barre.width) / 0.76))
    ecrire(Int(L), Int(H), ctx, a[5])
}
SWIFT

# Le symbole mal écrit : la barre par défaut, le volume en « speaker.wave.2.fil ». Sans cliquer Save : le refus
# d'enregistrer vient de Rust, en français seulement.
cp exemples/barre.toml "$dossier/barre.toml"
{ prelude_js "$dossier"; cat <<'JS'
(async () => {
  await attendre(1500);
  const i = barre.element.findIndex(e => e.type === 'slider' && e.reglage === 'volume');
  barre.element[i].symbole = 'speaker.wave.2.fil';
  demanderGlyphes(['speaker.wave.2.fil']);
  await attendre(800); selectionner({ i }); document.activeElement?.blur();
  await attendre(1200); capture('erreur.png');
  await attendre(1200); await finir();
})();
JS
} > "$dossier/pilote.js"
piloter "$dossier/barre.toml" "$dossier/pilote.js" "$dossier"
sips -Z 1600 "$dossier/erreur.png" --out docs/editeur-erreur-sombre-en.png >/dev/null

if command -v pngquant >/dev/null; then
    pngquant --force --skip-if-larger --quality 80-95 --ext .png docs/hero.png docs/social-preview.png docs/editeur-erreur-sombre-en.png || true
fi
ls -l docs/hero.png docs/social-preview.png docs/editeur-erreur-sombre-en.png
