# Claude vocal et Touch Bar

Tu appuies sur ✳ Claude dans la Touch Bar, tu parles, Claude répond à voix haute et en notification. Un clic sur la notification ouvre la conversation.

Tout tourne en local sauf la réponse : la transcription passe par whisper.cpp, la voix par Piper, la réponse par Claude Code (`claude -p`), donc avec ton compte Claude.

## Installer

Il faut [Homebrew](https://brew.sh) et Claude Code (la commande `claude`, connectée à ton compte). Depuis le dossier du dépôt Tactile :

```
extras/claude-voix/installer.sh
```

Le script installe `sox`, `whisper-cpp`, `media-control`, `terminal-notifier` (Homebrew) et `piper-tts` (`uv tool install`), télécharge depuis Hugging Face le modèle whisper `ggml-small-q5_1.bin` (181 Mo) et la voix `fr_FR-siwis-medium`, puis copie les fichiers aux endroits du tableau ci-dessous. Il se relance sans risque : rien n'est retéléchargé, et tes `prompt.md` et `reglages.json` déjà présents ne sont pas écrasés.

Ajoute ensuite à `~/.config/tactile/barre.toml` les deux éléments de `exemples/claude-vocal.toml` : l'état (🎙 / ⋯ / 🔊) et le bouton ✳ Claude.

## Le micro

macOS accorde le micro à l'app qui lance l'enregistrement, pas au script. Lancé depuis la Touch Bar, `claude-voix` est un processus de Tactile.app : c'est donc Tactile qui demande l'accès au micro la première fois (elle le déclare dans son `Info.plist`). Accepte, et c'est réglé ; tu peux le retirer dans Réglages Système → Confidentialité et sécurité → Micro.

Les anciennes versions de ce script passaient par une petite app intermédiaire, parce que la barre qui les lançait (MTMR) n'avait pas le droit au micro. Ce détour n'est plus utile : le script lance l'écoute directement.

Lancé depuis un terminal (`claude-voix --test`, par exemple), c'est le terminal qui demande le micro.

## Vérifier que tout marche

```
claude-voix --test
```

Rejoue une phrase synthétique (voix « Thomas » de macOS) à travers toute la chaîne (transcription, réponse, voix) et affiche le temps de chaque étape. Doit finir par `OK`.

## Où sont les pièces

| Quoi | Où |
|---|---|
| Script principal | `~/.local/bin/claude-voix` |
| Style des réponses (à retoucher librement) | `~/.config/claude-voix/prompt.md` |
| Garde-fous (interdits, actions à demande explicite) | `~/.config/claude-voix/reglages.json` |
| Modèle de transcription (whisper small, 181 Mo) | `~/.local/share/claude-voix/modeles/` |
| Voix Piper | `~/.local/share/claude-voix/voix/` |
| Conversation, audio, état de la barre | `/tmp/claude-voix/` (vidé à chaque redémarrage) |

## Gestes

- Appui sur ✳ Claude : l'écoute démarre. Nouvel appui : la question part chez Claude. Pendant la réponse, un appui coupe la voix.
- Pour annuler (rien n'est transcrit ni envoyé) : appui long sur ✳ Claude, ou toucher l'indicateur (🎙, ⋯) à sa gauche. Marche pendant l'écoute et pendant que Claude réfléchit. En ligne de commande : `claude-voix --annuler`.
- L'écoute ne s'arrête pas au silence : elle dure jusqu'au prochain appui, 5 minutes au maximum.
- Conversation : les 3 derniers échanges servent de contexte s'ils datent de moins de 10 minutes.

## Réglages

- Claude répond avec `--permission-mode auto` : il peut agir sur le Mac, dans les limites de `reglages.json` (commandes interdites, actions à ne faire que sur demande explicite) et de `prompt.md`. Relis ces deux fichiers avant le premier usage et durcis-les si besoin.
- `reglages.json` exclut ton `~/.claude/CLAUDE.md` personnel pour garder des réponses courtes ; retire la ligne `claudeMdExcludes` si tu veux qu'il s'applique.
- Changer de voix : télécharge une autre voix Piper (par exemple `fr_FR-tom-medium`, masculine, même dossier sur Hugging Face : `rhasspy/piper-voices`, `fr/fr_FR/tom/medium/`) dans `~/.local/share/claude-voix/voix/`, puis remplace `siwis` par `tom` dans la variable `VOIX` du script.
