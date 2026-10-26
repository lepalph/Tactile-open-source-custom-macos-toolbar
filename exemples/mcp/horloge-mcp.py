#!/usr/bin/env python3
"""Serveur MCP de démonstration (stdio, sans dépendance) : un outil `heure` qui renvoie {"heure": "HH:MM:SS"}.

Il parle les deux époques du protocole : moderne (2026-07-28, `server/discover` et `_meta` à chaque requête)
et, avec `--ancien`, seulement l'ancienne poignée de main `initialize` (2025-11-25). Il s'arrête quand son
entrée se ferme. Utilisé par exemples/mcp.toml et par les tests du client MCP de Tactile.
"""
import json
import sys
import time

ANCIEN = "--ancien" in sys.argv
OUTIL = {"name": "heure", "description": "L'heure locale", "inputSchema": {"type": "object", "properties": {}}}


def repondre(ident, resultat=None, erreur=None):
    message = {"jsonrpc": "2.0", "id": ident}
    message.update({"error": erreur} if erreur else {"result": resultat})
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


for ligne in sys.stdin:
    try:
        requete = json.loads(ligne)
    except ValueError:
        continue
    methode, ident = requete.get("method"), requete.get("id")
    if ident is None:
        continue  # notification (notifications/initialized…)
    if methode == "server/discover" and not ANCIEN:
        repondre(ident, {"resultType": "complete", "supportedVersions": ["2026-07-28"], "capabilities": {"tools": {}}})
    elif methode == "initialize":
        repondre(ident, {"protocolVersion": "2025-11-25", "capabilities": {"tools": {}}, "serverInfo": {"name": "horloge", "version": "1"}})
    elif methode == "tools/list":
        repondre(ident, {"tools": [OUTIL]})
    elif methode == "tools/call" and requete["params"].get("name") == "heure":
        texte = json.dumps({"heure": time.strftime("%H:%M:%S")})
        repondre(ident, {"content": [{"type": "text", "text": texte}]})
    elif methode == "tools/call":
        repondre(ident, {"content": [{"type": "text", "text": "outil inconnu"}], "isError": True})
    else:
        repondre(ident, erreur={"code": -32601, "message": "Method not found"})
