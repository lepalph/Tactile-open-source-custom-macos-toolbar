#!/bin/zsh
# Compose les visuels du dépôt à partir des captures existantes :
#   docs/hero.png            (1600 px de large) : la Touch Bar au-dessus de la fenêtre de l'éditeur ;
#   docs/social-preview.png  (1280×640) : l'image d'aperçu GitHub, à déposer à la main dans Settings.
# Outils système seulement : swift (CoreGraphics, CoreText) ; pngquant s'il est là, pour alléger.
set -euo pipefail
cd "${0:A:h}/.."

swift - docs/barre.png docs/editeur-sombre.png assets/icone-1024.png docs/hero.png docs/social-preview.png <<'SWIFT'
import AppKit

let a = CommandLine.arguments
func charger(_ chemin: String) -> CGImage {
    let src = CGImageSourceCreateWithURL(URL(fileURLWithPath: chemin) as CFURL, nil)!
    return CGImageSourceCreateImageAtIndex(src, 0, nil)!
}
let (barre, icone) = (charger(a[1]), charger(a[3]))
// Le bas de la capture de l'éditeur est vide : on le coupe sous la barre latérale (la fenêtre est réarrondie plus bas).
let pleine = charger(a[2])
let editeur = pleine.cropping(to: CGRect(x: 0, y: 0, width: pleine.width, height: pleine.height * 92 / 100))!

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
    ctx.draw(barre, in: CGRect(x: r.midX - l / 2, y: r.midY - h / 2, width: l, height: h))
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

if command -v pngquant >/dev/null; then
    pngquant --force --skip-if-larger --quality 80-95 --ext .png docs/hero.png docs/social-preview.png || true
fi
ls -l docs/hero.png docs/social-preview.png
