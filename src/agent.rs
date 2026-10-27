//! Tactile comme serveur MCP : un agent (Claude Code) écrit sur la Touch Bar.
//!
//! Deux moitiés. `tactile --mcp` parle JSON-RPC 2.0 sur stdin/stdout (une ligne par message) et relaie chaque
//! appel d'outil à l'app qui tourne par une socket Unix (`~/Library/Application Support/Tactile/agent.sock`) :
//! une requête JSON par connexion, réponse en une ligne, 64 Ko au plus. L'app écoute (`ecouter`) et confie
//! la requête à `barre::agent`, qui l'affiche sur le fil principal. Rien de ce qui passe par la socket n'exécute
//! une commande : deux outils, du texte à l'écran et un libellé en retour.

use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MODERNE: &str = "2026-07-28";
const ANCIENNE: &str = "2025-11-25";
/// Taille maximale d'une requête (et d'une réponse) sur la socket.
const MAX_SOCKET: u64 = 64 * 1024;
const MAX_TEXTE: usize = 80;
const MAX_LIBELLE: usize = 20;
const DUREE_DEFAUT: u64 = 5;
const DELAI_DEFAUT: u64 = 120;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Requete {
    Afficher { texte: String, duree_s: u64 },
    Demander { question: String, choix: Vec<String>, delai_s: u64 },
}

/// Coupe à `max` caractères (« … » en dernier), retours à la ligne et tabulations comptés comme des espaces.
fn tronquer(s: &str, max: usize) -> String {
    let s: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max - 1).collect::<String>().trim_end().to_string() + "…"
}

fn texte(args: &Value, cle: &str, max: usize) -> Result<String, String> {
    let t = args[cle].as_str().ok_or_else(|| format!("« {cle} » (texte) est requis"))?;
    match tronquer(t, max) {
        t if t.is_empty() => Err(format!("« {cle} » est vide")),
        t => Ok(t),
    }
}

fn entier(args: &Value, cle: &str, defaut: u64, min: u64, max: u64) -> Result<u64, String> {
    match &args[cle] {
        Value::Null => Ok(defaut),
        v => v.as_u64().filter(|n| (min..=max).contains(n)).ok_or_else(|| format!("« {cle} » doit être un entier de {min} à {max}")),
    }
}

impl Requete {
    /// Valide un appel d'outil (`nom`, `arguments`) : les mêmes règles côté `--mcp` et côté app (la socket est un bord de confiance).
    pub(crate) fn depuis_outil(nom: &str, args: &Value) -> Result<Requete, String> {
        match nom {
            "afficher" => Ok(Requete::Afficher { texte: texte(args, "texte", MAX_TEXTE)?, duree_s: entier(args, "duree_s", DUREE_DEFAUT, 1, 60)? }),
            "demander" => {
                let choix = args["choix"].as_array().ok_or("« choix » (liste de 1 à 4 libellés) est requis")?;
                if !(1..=4).contains(&choix.len()) {
                    return Err("« choix » doit contenir de 1 à 4 libellés".into());
                }
                let choix = choix.iter().map(|c| texte(&json!({ "choix": c }), "choix", MAX_LIBELLE)).collect::<Result<Vec<_>, _>>()?;
                Ok(Requete::Demander { question: texte(args, "question", MAX_TEXTE)?, choix, delai_s: entier(args, "delai_s", DELAI_DEFAUT, 5, 600)? })
            }
            _ => Err(format!("outil inconnu « {nom} »")),
        }
    }

    fn nom(&self) -> &'static str {
        match self {
            Requete::Afficher { .. } => "afficher",
            Requete::Demander { .. } => "demander",
        }
    }

    /// Combien de temps la requête peut durer, pour le délai de lecture de la réponse.
    pub(crate) fn duree_max(&self) -> u64 {
        match self {
            Requete::Afficher { duree_s, .. } => *duree_s,
            Requete::Demander { delai_s, .. } => *delai_s,
        }
    }

    fn en_json(&self) -> Value {
        match self {
            Requete::Afficher { texte, duree_s } => json!({ "action": self.nom(), "texte": texte, "duree_s": duree_s }),
            Requete::Demander { question, choix, delai_s } => json!({ "action": self.nom(), "question": question, "choix": choix, "delai_s": delai_s }),
        }
    }
}

// ---------- Serveur MCP (stdio) ----------

fn outils() -> Value {
    json!([
        {
            "name": "afficher",
            "description": "Affiche un court message sur la Touch Bar de l'utilisateur, puis la barre normale revient. Ne bloque pas.",
            "inputSchema": { "type": "object", "properties": {
                "texte": { "type": "string", "description": "Le message (80 caractères au plus)." },
                "duree_s": { "type": "integer", "minimum": 1, "maximum": 60, "default": 5, "description": "Secondes d'affichage." }
            }, "required": ["texte"] }
        },
        {
            "name": "demander",
            "description": "Pose une question sur la Touch Bar avec 1 à 4 boutons et attend que l'utilisateur en touche un. Renvoie le libellé touché, ou « aucun » (délai écoulé ou ✕). Bloque jusqu'à la réponse.",
            "inputSchema": { "type": "object", "properties": {
                "question": { "type": "string", "description": "La question (80 caractères au plus)." },
                "choix": { "type": "array", "items": { "type": "string" }, "minItems": 1, "maxItems": 4, "description": "Libellés des boutons (20 caractères au plus)." },
                "delai_s": { "type": "integer", "minimum": 5, "maximum": 600, "default": 120, "description": "Secondes avant « aucun »." }
            }, "required": ["question", "choix"] }
        }
    ])
}

/// Une ligne JSON-RPC entrante → la réponse à écrire (`None` pour une notification).
/// `appeler` relaie une requête validée à l'app : `Ok(texte)` ou `Err(message d'outil)`.
fn traiter(ligne: &str, appeler: &impl Fn(&Requete) -> Result<String, String>) -> Option<Value> {
    let Ok(msg) = serde_json::from_str::<Value>(ligne) else {
        return Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "JSON illisible" } }));
    };
    let id = msg.get("id").filter(|i| !i.is_null())?.clone();
    let erreur = |code: i64, message: &str| Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }));
    let resultat = |r: Value| Some(json!({ "jsonrpc": "2.0", "id": id, "result": r }));
    let params = &msg["params"];
    match msg["method"].as_str() {
        Some("initialize") => {
            let demandee = params["protocolVersion"].as_str().unwrap_or("");
            let version = if demandee == MODERNE || demandee == ANCIENNE { demandee } else { ANCIENNE };
            resultat(json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "tactile", "version": env!("CARGO_PKG_VERSION") }
            }))
        }
        Some("ping") => resultat(json!({})),
        Some("tools/list") => resultat(json!({ "tools": outils() })),
        Some("tools/call") => {
            let Some(nom) = params["name"].as_str() else { return erreur(-32602, "« name » manquant") };
            if !["afficher", "demander"].contains(&nom) {
                return erreur(-32602, &format!("outil inconnu « {nom} »"));
            }
            let suite = Requete::depuis_outil(nom, &params["arguments"]).and_then(|r| appeler(&r));
            let (texte, echec) = match suite {
                Ok(t) => (t, false),
                Err(e) => (e, true),
            };
            resultat(json!({ "content": [{ "type": "text", "text": texte }], "isError": echec }))
        }
        Some(m) => erreur(-32601, &format!("méthode inconnue « {m} »")),
        None => erreur(-32600, "requête invalide"),
    }
}

/// La boucle de `tactile --mcp` : une ligne par message, réponse en une ligne.
pub(crate) fn serveur(entree: impl BufRead, mut sortie: impl Write, appeler: impl Fn(&Requete) -> Result<String, String>) {
    for ligne in entree.lines() {
        let Ok(ligne) = ligne else { break };
        if ligne.trim().is_empty() {
            continue;
        }
        if let Some(reponse) = traiter(&ligne, &appeler)
            && writeln!(sortie, "{reponse}").and_then(|_| sortie.flush()).is_err()
        {
            break;
        }
    }
}

/// `tactile --mcp`.
pub(crate) fn lancer_mcp() {
    serveur(std::io::stdin().lock(), std::io::stdout().lock(), |r| envoyer(&chemin_socket(), r));
}

// ---------- Socket ----------

pub(crate) fn chemin_socket() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Tactile/agent.sock")
}

/// Côté `--mcp` : une connexion, une requête, une réponse. App absente = erreur d'outil claire.
pub(crate) fn envoyer(socket: &Path, requete: &Requete) -> Result<String, String> {
    let mut flux = UnixStream::connect(socket).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => "Tactile n'est pas lancé : ouvre l'app Tactile, puis réessaie.".to_string(),
        _ => format!("Tactile ne répond pas ({e})"),
    })?;
    let _ = flux.set_read_timeout(Some(Duration::from_secs(requete.duree_max() + 15)));
    writeln!(flux, "{}", requete.en_json()).map_err(|e| format!("Tactile ne répond pas ({e})"))?;
    let mut ligne = Vec::new();
    std::io::BufReader::new(&flux).take(MAX_SOCKET).read_until(b'\n', &mut ligne).map_err(|e| format!("Tactile ne répond pas ({e})"))?;
    let reponse: Value = serde_json::from_slice(&ligne).map_err(|_| "Tactile n'a pas répondu".to_string())?;
    match (reponse["ok"].as_str(), reponse["erreur"].as_str()) {
        (Some(ok), _) => Ok(ok.to_string()),
        (_, Some(e)) => Err(e.to_string()),
        _ => Err("réponse de Tactile illisible".into()),
    }
}

/// Une connexion côté app : lit une requête (bornée, 5 s), la valide, la traite, répond en une ligne.
fn servir(mut flux: UnixStream, traiter: &impl Fn(Requete) -> Result<String, String>) {
    let _ = flux.set_read_timeout(Some(Duration::from_secs(5)));
    let mut ligne = Vec::new();
    let lu = std::io::BufReader::new(&flux).take(MAX_SOCKET + 1).read_until(b'\n', &mut ligne);
    let resultat = match lu {
        Ok(_) if ligne.len() as u64 > MAX_SOCKET => Err("requête de plus de 64 Ko".to_string()),
        Ok(_) => serde_json::from_slice::<Value>(&ligne)
            .map_err(|_| "requête illisible".to_string())
            .and_then(|v| Requete::depuis_outil(v["action"].as_str().unwrap_or(""), &v))
            .and_then(traiter),
        Err(e) => Err(format!("requête non reçue ({e})")),
    };
    let reponse = match resultat {
        Ok(ok) => json!({ "ok": ok }),
        Err(erreur) => json!({ "erreur": erreur }),
    };
    let _ = writeln!(flux, "{reponse}");
}

/// Crée le dossier (0700) et la socket (0600, recréée), puis écoute dans des fils : une connexion = un fil,
/// pour qu'une question en attente n'empêche pas de répondre « occupé » à la suivante.
pub(crate) fn ecouter(socket: &Path, traiter: impl Fn(Requete) -> Result<String, String> + Send + Sync + 'static) -> std::io::Result<()> {
    if let Some(dossier) = socket.parent() {
        std::fs::create_dir_all(dossier)?;
        std::fs::set_permissions(dossier, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    let ecouteur = UnixListener::bind(socket)?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))?;
    let traiter = std::sync::Arc::new(traiter);
    std::thread::Builder::new().name("tactile-agent".into()).spawn(move || {
        for flux in ecouteur.incoming().flatten() {
            let traiter = traiter.clone();
            std::thread::spawn(move || servir(flux, &*traiter));
        }
    })?;
    Ok(())
}

/// À la fermeture de l'app : la socket ne doit pas survivre.
pub(crate) fn nettoyer() {
    let _ = std::fs::remove_file(chemin_socket());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(_: &Requete) -> Result<String, String> {
        Ok("fait".into())
    }

    fn appel(msg: Value) -> Value {
        traiter(&msg.to_string(), &ok).expect("réponse")
    }

    #[test]
    fn initialize_renvoie_la_version_connue_sinon_2025() {
        for (demandee, attendue) in [("2025-11-25", "2025-11-25"), ("2026-07-28", "2026-07-28"), ("2024-11-05", "2025-11-25")] {
            let r = appel(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": demandee } }));
            assert_eq!(r["result"]["protocolVersion"], attendue);
            assert_eq!(r["id"], 1);
            assert_eq!(r["result"]["serverInfo"]["name"], "tactile");
        }
        assert_eq!(appel(json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize" }))["result"]["protocolVersion"], ANCIENNE);
    }

    #[test]
    fn notifications_ping_liste_et_erreurs() {
        assert!(traiter(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, &ok).is_none());
        assert_eq!(appel(json!({ "id": "a", "method": "ping" }))["result"], json!({}));
        let liste = appel(json!({ "id": 3, "method": "tools/list" }));
        let noms: Vec<_> = liste["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(noms, ["afficher", "demander"]);
        assert_eq!(appel(json!({ "id": 4, "method": "truc" }))["error"]["code"], -32601);
        assert_eq!(appel(json!({ "id": 5, "method": "tools/call", "params": { "name": "x" } }))["error"]["code"], -32602);
        assert_eq!(traiter("pas du json", &ok).unwrap()["error"]["code"], -32700);
    }

    #[test]
    fn appel_d_outil_valide_ou_refuse() {
        let r = appel(json!({ "id": 1, "method": "tools/call", "params": { "name": "afficher", "arguments": { "texte": "Salut" } } }));
        assert_eq!(r["result"]["isError"], false);
        assert_eq!(r["result"]["content"][0]["text"], "fait");
        let r = appel(json!({ "id": 2, "method": "tools/call", "params": { "name": "afficher", "arguments": { "texte": "Salut", "duree_s": 61 } } }));
        assert_eq!(r["result"]["isError"], true);
        assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("duree_s"));
    }

    #[test]
    fn validation_des_arguments() {
        let long = "é".repeat(100);
        match Requete::depuis_outil("afficher", &json!({ "texte": long })).unwrap() {
            Requete::Afficher { texte, duree_s } => assert_eq!((texte.chars().count(), duree_s, texte.ends_with('…')), (80, 5, true)),
            r => panic!("{r:?}"),
        }
        let d = Requete::depuis_outil("demander", &json!({ "question": "Ok ?\nOui", "choix": ["Oui", " Non ", "x".repeat(30)] })).unwrap();
        let Requete::Demander { question, choix, delai_s } = d else { panic!() };
        assert_eq!((question.as_str(), delai_s, choix[1].as_str(), choix[2].chars().count()), ("Ok ? Oui", 120, "Non", 20));
        for mauvais in [
            ("afficher", json!({})),
            ("afficher", json!({ "texte": "  " })),
            ("afficher", json!({ "texte": "a", "duree_s": 0 })),
            ("afficher", json!({ "texte": "a", "duree_s": "5" })),
            ("demander", json!({ "question": "q", "choix": [] })),
            ("demander", json!({ "question": "q", "choix": ["1", "2", "3", "4", "5"] })),
            ("demander", json!({ "question": "q", "choix": [""] })),
            ("demander", json!({ "question": "q", "choix": [3] })),
            ("demander", json!({ "question": "q", "choix": ["a"], "delai_s": 4 })),
            ("demander", json!({ "question": "q", "choix": ["a"], "delai_s": 601 })),
            ("demander", json!({ "choix": ["a"] })),
            ("autre", json!({})),
        ] {
            assert!(Requete::depuis_outil(mauvais.0, &mauvais.1).is_err(), "{mauvais:?}");
        }
    }

    #[test]
    fn boucle_stdio_une_ligne_par_message() {
        let entree = "{\"id\":1,\"method\":\"ping\"}\n\n{\"method\":\"notifications/initialized\"}\n{\"id\":2,\"method\":\"ping\"}\n";
        let mut sortie = Vec::new();
        serveur(entree.as_bytes(), &mut sortie, ok);
        let lignes: Vec<_> = String::from_utf8(sortie).unwrap().lines().map(String::from).collect();
        assert_eq!(lignes.len(), 2);
        assert!(lignes[1].contains("\"id\":2"));
    }

    #[test]
    fn socket_de_bout_en_bout_sans_barre() {
        // /tmp : le chemin d'une socket Unix est limité à ~100 octets.
        let d = PathBuf::from(format!("/tmp/tactile-agent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let s = d.join("agent.sock");
        // Pas d'app : message clair.
        assert!(envoyer(&s, &Requete::Afficher { texte: "a".into(), duree_s: 1 }).unwrap_err().contains("n'est pas lancé"));
        ecouter(&s, |r| match r {
            Requete::Demander { choix, .. } => Ok(choix[1].clone()),
            Requete::Afficher { .. } => Err("occupé".into()),
        })
        .unwrap();
        assert_eq!(std::fs::metadata(&d).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::metadata(&s).unwrap().permissions().mode() & 0o777, 0o600);
        let demande = Requete::Demander { question: "q".into(), choix: vec!["Oui".into(), "Non".into()], delai_s: 5 };
        assert_eq!(envoyer(&s, &demande).unwrap(), "Non");
        assert_eq!(envoyer(&s, &Requete::Afficher { texte: "a".into(), duree_s: 1 }).unwrap_err(), "occupé");
        // Une requête invalide ou trop grosse est refusée côté app.
        let brut = |octets: Vec<u8>| {
            let mut c = UnixStream::connect(&s).unwrap();
            let _ = c.write_all(&octets);
            let mut r = String::new();
            std::io::BufReader::new(c).read_line(&mut r).unwrap();
            serde_json::from_str::<Value>(&r).unwrap()
        };
        assert!(brut(b"{\"action\":\"afficher\",\"texte\":\"a\",\"duree_s\":99}\n".to_vec())["erreur"].as_str().unwrap().contains("duree_s"));
        assert!(brut(vec![b'a'; 70_000])["erreur"].as_str().unwrap().contains("64 Ko"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
