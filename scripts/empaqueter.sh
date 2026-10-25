#!/bin/zsh
# Construit target/Tactile.app universelle (Apple Silicon + Intel, signée ad hoc) et target/Tactile.zip.
set -euo pipefail
cd "${0:A:h}/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[[ -n $version ]] || { print -u2 "version introuvable dans Cargo.toml"; exit 1 }

# Rust installé par rustup, ou par mise sans activation dans le shell.
outil() { if command -v $1 >/dev/null; then "$@"; else mise exec -- "$@"; fi }

cibles=(aarch64-apple-darwin x86_64-apple-darwin)
outil rustup target add $cibles >/dev/null
for cible in $cibles; do
  outil cargo build --release --target $cible
done

app=target/Tactile.app
rm -rf $app target/Tactile.zip
mkdir -p $app/Contents/MacOS
lipo -create -output $app/Contents/MacOS/tactile target/${^cibles}/release/tactile

cat > $app/Contents/Info.plist <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleIdentifier</key>
	<string>fr.palph.tactile</string>
	<key>CFBundleName</key>
	<string>Tactile</string>
	<key>CFBundleDisplayName</key>
	<string>Tactile</string>
	<key>CFBundleExecutable</key>
	<string>tactile</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>$version</string>
	<key>CFBundleVersion</key>
	<string>$version</string>
	<key>LSUIElement</key>
	<true/>
	<key>LSMinimumSystemVersion</key>
	<string>13.0</string>
	<key>NSMicrophoneUsageDescription</key>
	<string>Tactile utilise le micro quand un bouton de ta barre lance une commande vocale.</string>
</dict>
</plist>
PLIST
plutil -lint -s $app/Contents/Info.plist

codesign --force --deep -s - $app
ditto -c -k --keepParent $app target/Tactile.zip

print "$PWD/$app"
