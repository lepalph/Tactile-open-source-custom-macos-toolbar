#!/bin/zsh
# Vérifie toutes les fonctions de la barre sur la vraie Touch Bar : lance Tactile en autotest
# (chaque contrôle est actionné par le même chemin qu'un doigt), capture la barre 1,2 s après
# chaque étape dans target/verification/, compare chaque capture à l'écran attendu
# (barre principale, sous-barre ou barre d'Apple) et échoue au moindre écart. L'écran doit être allumé.
set -uo pipefail
cd "${0:A:h}/.."

outil() { if command -v $1 >/dev/null; then "$@"; else mise exec -- "$@"; fi }
outil cargo build --release -q || exit 1

dossier=target/verification
rm -rf $dossier && mkdir -p $dossier
journal=$dossier/journal.log

pkill -x tactile; sleep 0.5
TACTILE_AUTOTEST=1 target/release/tactile 2>$journal &
pid=$!

vus=0
for i in {1..400}; do
  sleep 0.1
  n=$(grep -c '^AUTOTEST [0-9]* pas' $journal)
  if (( n > vus )); then
    vus=$n
    sleep 1.2
    attendu=$(grep '^AUTOTEST [0-9]* pas' $journal | tail -1 | sed 's/.*attendu=//')
    screencapture -b $dossier/etape-$(printf %02d $((n - 1)))-$attendu.png
  fi
  grep -q '^AUTOTEST fin' $journal && break
done
kill $pid 2>/dev/null

# Classe chaque capture d'après sa zone de gauche : « principale » ressemble à l'étape 0,
# « sous » à l'étape 1 (bouton ✕), le reste est la barre d'Apple.
visuel=$(swift - $dossier/etape-*.png <<'SWIFT'
import AppKit
func pixels(_ f: String) -> [Double] {
    let r = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: f)))!
    var v: [Double] = []
    for x in stride(from: 0, to: 130, by: 3) { for y in stride(from: 0, to: r.pixelsHigh, by: 3) { v.append(Double(r.colorAt(x: x, y: y)?.brightnessComponent ?? 0)) } }
    return v
}
func ecart(_ a: [Double], _ b: [Double]) -> Double { zip(a, b).map { abs($0 - $1) }.reduce(0, +) / Double(a.count) }
let fichiers = Array(CommandLine.arguments.dropFirst())
let principale = pixels(fichiers[0]), sous = pixels(fichiers[1])
// Le Control Strip d'Apple occupe la droite ; s'il est noir, l'écran était inactif et rien n'est prouvé.
func droiteAllumee(_ f: String) -> Bool {
    let r = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: f)))!
    var n = 0
    for x in stride(from: 1390, to: r.pixelsWide, by: 4) { for y in stride(from: 0, to: r.pixelsHigh, by: 4) { if (r.colorAt(x: x, y: y)?.brightnessComponent ?? 0) > 0.1 { n += 1 } } }
    return n > 50
}
// L'élément « ⚠ config ligne N » est rouge, en tête de barre.
func rougeAGauche(_ f: String) -> Bool {
    let r = NSBitmapImageRep(data: try! Data(contentsOf: URL(fileURLWithPath: f)))!
    var n = 0
    for x in stride(from: 0, to: 300, by: 3) { for y in stride(from: 0, to: r.pixelsHigh, by: 3) {
        if let c = r.colorAt(x: x, y: y), c.redComponent > 0.4, c.greenComponent < 0.25, c.blueComponent < 0.3 { n += 1 }
    } }
    return n > 100
}
for f in fichiers {
    let p = pixels(f)
    var vu = rougeAGauche(f) ? "erreur" : ecart(p, principale) < 0.02 ? "principale" : ecart(p, sous) < 0.02 ? "sous" : "apple"
    if vu == "apple" && !droiteAllumee(f) { vu = "noir" }
    let attendu = String(f.split(separator: "-").last!.dropLast(4))
    let verdict = vu == attendu ? "ok   " : vu == "noir" ? "?    " : "ÉCHEC"
    print("\(verdict) \((f as NSString).lastPathComponent) : vu \(vu == "noir" ? "écran noir, non vérifiable" : vu)")
}
SWIFT
)

grep '^AUTOTEST' $journal | grep -v ' pas '
print -- "--- écran"
print -- $visuel
if grep -q '^AUTOTEST fin 0 échec' $journal && ! print -- $visuel | grep -q ÉCHEC; then
  if print -- $visuel | grep -q '^?'; then
    print "Aucun échec, mais des écrans n'ont pas pu être vus (Touch Bar inactive) : touche le trackpad pendant la vérification."
    exit 2
  fi
  print "Tout est vérifié. Captures : $dossier"
else
  print -u2 "Vérification échouée : voir $journal et les captures de $dossier"
  exit 1
fi
