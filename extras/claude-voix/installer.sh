#!/bin/zsh
# Installe Claude vocal : dépendances, modèle whisper, voix piper, script et réglages.
# Relançable sans risque : rien n'est retéléchargé, prompt.md et reglages.json existants sont gardés.
set -euo pipefail

ICI=${0:A:h}
CONF=~/.config/claude-voix
DATA=~/.local/share/claude-voix
HF=https://huggingface.co

command -v brew >/dev/null || { print "Homebrew est nécessaire : https://brew.sh"; exit 1 }
command -v claude >/dev/null || print "Attention : la commande claude (Claude Code) est introuvable ; installe-la avant d'utiliser Claude vocal."

brew install sox whisper-cpp media-control terminal-notifier
command -v uv >/dev/null || brew install uv
uv tool install piper-tts

telecharger() {
  [[ -s $2 ]] && return
  print "Téléchargement de ${2:t}…"
  curl -fL --progress-bar -o "$2.part" "$1" && mv "$2.part" "$2"
}

mkdir -p ~/.local/bin $CONF $DATA/modeles $DATA/voix
telecharger $HF/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin $DATA/modeles/ggml-small-q5_1.bin
for f in fr_FR-siwis-medium.onnx fr_FR-siwis-medium.onnx.json; do
  telecharger $HF/rhasspy/piper-voices/resolve/main/fr/fr_FR/siwis/medium/$f $DATA/voix/$f
done

install -m 755 $ICI/claude-voix ~/.local/bin/claude-voix
install -m 644 $ICI/README.md $CONF/README.md
[[ -e $CONF/prompt.md ]] || install -m 644 $ICI/prompt.md $CONF/prompt.md
# claudeMdExcludes attend un chemin absolu.
[[ -e $CONF/reglages.json ]] || sed "s|\"~/.claude/CLAUDE.md\"|\"$HOME/.claude/CLAUDE.md\"|" $ICI/reglages.json >$CONF/reglages.json

print "Installé. Vérifie la chaîne avec : ~/.local/bin/claude-voix --test"
print "Puis ajoute les deux éléments de exemples/claude-vocal.toml à ~/.config/tactile/barre.toml."
