#!/bin/zsh
# Dessine l'icône de Tactile en code et produit assets/icone-1024.png puis assets/Tactile.icns.
# Outils système seulement : swift (CoreGraphics), sips, iconutil.
set -euo pipefail
cd "${0:A:h}/.."
mkdir -p assets

# Carré arrondi anthracite → noir sur la grille d'icônes macOS (824 px dans 1024),
# une bande sombre (la Touch Bar) et trois pastilles, celle du milieu orange brique.
# shortcut: la bande garde l'esprit de 1004:60, pas le ratio exact (il ferait moins d'1 px à 16 px) ;
# à revoir si une déclinaison grande taille seule est voulue.
swift - assets/icone-1024.png <<'SWIFT'
import AppKit

let taille: CGFloat = 1024
let ctx = CGContext(data: nil, width: Int(taille), height: Int(taille), bitsPerComponent: 8, bytesPerRow: 0,
                    space: CGColorSpace(name: CGColorSpace.sRGB)!,
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

func couleur(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
}
func arrondi(_ r: CGRect, _ rayon: CGFloat) -> CGPath { CGPath(roundedRect: r, cornerWidth: rayon, cornerHeight: rayon, transform: nil) }

// Carré arrondi avec ombre portée, puis dégradé vertical.
let carre = CGRect(x: 100, y: 100, width: 824, height: 824)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 28, color: couleur(0x000000, 0.45))
ctx.addPath(arrondi(carre, 184)); ctx.setFillColor(couleur(0x1A1A1C)); ctx.fillPath()
ctx.restoreGState()
ctx.saveGState()
ctx.addPath(arrondi(carre, 184)); ctx.clip()
let degrade = CGGradient(colorsSpace: ctx.colorSpace, colors: [couleur(0x3A3A3C), couleur(0x0C0C0E)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(degrade, start: CGPoint(x: 512, y: carre.maxY), end: CGPoint(x: 512, y: carre.minY), options: [])
ctx.restoreGState()
// Liseré clair en haut, pour détacher le carré d'un fond sombre.
ctx.addPath(arrondi(carre.insetBy(dx: 2, dy: 2), 182)); ctx.setStrokeColor(couleur(0xFFFFFF, 0.10)); ctx.setLineWidth(4); ctx.strokePath()

// La Touch Bar : bande noire centrée.
let bande = CGRect(x: 188, y: 427, width: 648, height: 170)
ctx.addPath(arrondi(bande, 44)); ctx.setFillColor(couleur(0x000000, 0.55)); ctx.fillPath()

// Trois pastilles : deux claires, l'accent orange brique du projet au milieu.
let pastilles: [CGColor] = [couleur(0xE5E5EA), couleur(0xB4552F), couleur(0xE5E5EA)]
for (i, c) in pastilles.enumerated() {
    let r = CGRect(x: bande.minX + 30 + CGFloat(i) * 204, y: bande.minY + 30, width: 180, height: 110)
    ctx.addPath(arrondi(r, 34)); ctx.setFillColor(c); ctx.fillPath()
}

let png = NSBitmapImageRep(cgImage: ctx.makeImage()!).representation(using: .png, properties: [:])!
try! png.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
SWIFT

jeu=$(mktemp -d)/Tactile.iconset
mkdir -p $jeu
for t in 16 32 128 256 512; do
  sips -z $t $t assets/icone-1024.png --out $jeu/icon_${t}x${t}.png >/dev/null
  sips -z $((t*2)) $((t*2)) assets/icone-1024.png --out $jeu/icon_${t}x${t}@2x.png >/dev/null
done
iconutil -c icns $jeu -o assets/Tactile.icns
rm -rf ${jeu:h}
print "assets/icone-1024.png assets/Tactile.icns"
