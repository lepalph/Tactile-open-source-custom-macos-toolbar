//! Client MCP minimal (JSON-RPC 2.0) : `tools/call` sur un serveur stdio gardé vivant, ou sur une adresse
//! Streamable HTTP. Double époque, comme le demande la spécification 2026-07-28 : sonde `server/discover`
//! (serveur moderne, `_meta` à chaque requête), sinon poignée de main `initialize` (serveurs 2025, encore les plus courants).

use super::{http, TAILLE_MAX};
use crate::barre::{inscrire, retirer, tuer_groupe, Processus};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Version moderne (métadonnées par requête).
const MODERNE: &str = "2026-07-28";
/// Version demandée à un serveur qui ne connaît que `initialize`.
const ANCIENNE: &str = "2025-11-25";

#[derive(Debug)]
pub(crate) enum Erreur {
    /// Le serveur a répondu une erreur (ou l'outil a échoué) : il reste vivant.
    Rpc(i64, String),
    /// Pas de réponse, processus mort, HTTP illisible : à relancer.
    Lien(String),
}

impl Erreur {
    pub(crate) fn relancer(&self) -> bool {
        matches!(self, Erreur::Lien(_))
    }
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Erreur::Rpc(code, message) if *code != 0 => write!(f, "serveur MCP : {message} ({code})"),
            Erreur::Rpc(_, message) => write!(f, "outil : {message}"),
            Erreur::Lien(message) => write!(f, "serveur MCP : {message}"),
        }
    }
}

enum Lien {
    Stdio { enfant: Child, entree: Option<ChildStdin>, lignes: Receiver<String>, processus: Processus },
    Http { url: String, entetes: Vec<(String, String)>, session: Option<String>, processus: Processus },
}

pub(crate) struct Client {
    lien: Lien,
    /// `None` : moderne ; sinon la version négociée par `initialize`.
    ancienne: Option<String>,
    id: u64,
    delai: Duration,
    /// Secrets des en-têtes (adresse HTTP), à masquer dans les erreurs.
    pub secrets: Vec<String>,
}

impl Client {
    /// Lance `commande` (zsh, son propre groupe de processus, tué avec la barre) et se connecte.
    pub(crate) fn stdio(commande: &str, processus: &Processus, delai: Duration) -> Result<Client, String> {
        let mut enfant = std::process::Command::new("/bin/zsh")
            .args(["-c", commande])
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Le serveur y écrit ses traces (bannière de npx…) : le journal n'en a que faire.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("zsh : {e}"))?;
        let pid = enfant.id() as i32;
        if !inscrire(processus, pid) {
            return Err("barre remplacée".into());
        }
        let sortie = enfant.stdout.take().expect("stdout demandé");
        let (envoi, lignes) = std::sync::mpsc::channel();
        // Le fil s'arrête à la fin de la sortie, quand le serveur meurt ou est tué. Chaque ligne est lue bornée
        // (pas `lines()`, qui garderait une ligne sans fin) : au-delà de 1 Mo, le serveur est tué.
        std::thread::spawn(move || {
            let mut lecteur = std::io::BufReader::new(sortie);
            loop {
                let mut ligne = Vec::new();
                match (&mut lecteur).take(TAILLE_MAX as u64 + 1).read_until(b'\n', &mut ligne) {
                    Ok(0) | Err(_) => break,
                    Ok(_) if ligne.len() > TAILLE_MAX => {
                        eprintln!("tactile : serveur MCP : réponse de plus de 1 Mo, serveur arrêté");
                        tuer_groupe(pid);
                        break;
                    }
                    Ok(_) => {
                        let fin = ligne.strip_suffix(b"\n").unwrap_or(&ligne);
                        let fin = fin.strip_suffix(b"\r").unwrap_or(fin);
                        if envoi.send(String::from_utf8_lossy(fin).into_owned()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let entree = enfant.stdin.take();
        let lien = Lien::Stdio { enfant, entree, lignes, processus: processus.clone() };
        Client::connecter(lien, Vec::new(), delai)
    }

    /// Adresse Streamable HTTP : chaque message est un POST (`curl`, comme une source http).
    pub(crate) fn http(url: &str, entetes: Vec<(String, String)>, processus: &Processus, delai: Duration) -> Result<Client, String> {
        if !super::url_permise(url) {
            return Err(format!("« {url} » refusée : seules https:// et http://localhost sont lues"));
        }
        let lien = Lien::Http { url: url.into(), entetes, session: None, processus: processus.clone() };
        Client::connecter(lien, Vec::new(), delai)
    }

    fn connecter(lien: Lien, secrets: Vec<String>, delai: Duration) -> Result<Client, String> {
        let mut client = Client { lien, ancienne: None, id: 0, delai, secrets };
        match client.demander("server/discover", json!({})) {
            Ok(r) if r["supportedVersions"].as_array().is_some_and(|v| v.iter().any(|v| v == MODERNE)) => return Ok(client),
            Ok(r) => return Err(format!("serveur MCP : versions {} seulement", r["supportedVersions"])),
            // Une erreur moderne connue : le serveur est moderne, ne pas retomber sur `initialize`.
            Err(Erreur::Rpc(code, message)) if (-32022..=-32020).contains(&code) => return Err(format!("serveur MCP : {message}")),
            Err(Erreur::Lien(e)) if client.mort() => return Err(format!("serveur MCP : {e}")),
            Err(_) => {}
        }
        client.ancienne = Some(ANCIENNE.into());
        let r = client.demander("initialize", json!({ "protocolVersion": ANCIENNE, "capabilities": {}, "clientInfo": { "name": "tactile", "version": env!("CARGO_PKG_VERSION") } })).map_err(|e| e.to_string())?;
        client.ancienne = Some(r["protocolVersion"].as_str().unwrap_or(ANCIENNE).to_string());
        client.notifier("notifications/initialized").map_err(|e| e.to_string())?;
        Ok(client)
    }

    /// Le serveur stdio est-il sorti ?
    fn mort(&mut self) -> bool {
        match &mut self.lien {
            Lien::Stdio { enfant, .. } => !matches!(enfant.try_wait(), Ok(None)),
            Lien::Http { .. } => false,
        }
    }

    /// `tools/call` : `structuredContent` s'il y en a, sinon `content[0].text` (JSON si possible, sinon sous `texte`).
    pub(crate) fn appeler(&mut self, outil: &str, arguments: Value) -> Result<Value, Erreur> {
        let r = self.demander("tools/call", json!({ "name": outil, "arguments": arguments }))?;
        resultat_outil(&r)
    }

    fn meta(&self) -> Value {
        json!({
            "io.modelcontextprotocol/protocolVersion": MODERNE,
            "io.modelcontextprotocol/clientInfo": { "name": "tactile", "version": env!("CARGO_PKG_VERSION") },
            "io.modelcontextprotocol/clientCapabilities": {},
        })
    }

    fn demander(&mut self, methode: &str, mut params: Value) -> Result<Value, Erreur> {
        self.id += 1;
        let id = self.id;
        if self.ancienne.is_none() {
            params["_meta"] = self.meta();
        }
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": methode, "params": params });
        let reponse = match self.lien {
            Lien::Stdio { .. } => {
                self.ecrire(&message)?;
                self.attendre(id)?
            }
            Lien::Http { .. } => self.poster(&message)?.ok_or_else(|| Erreur::Lien("réponse vide".into()))?,
        };
        if let Some(e) = reponse.get("error") {
            return Err(Erreur::Rpc(e["code"].as_i64().unwrap_or(-1), e["message"].as_str().unwrap_or("erreur").to_string()));
        }
        Ok(reponse["result"].clone())
    }

    fn notifier(&mut self, methode: &str) -> Result<(), Erreur> {
        let message = json!({ "jsonrpc": "2.0", "method": methode });
        match self.lien {
            Lien::Stdio { .. } => self.ecrire(&message),
            Lien::Http { .. } => self.poster(&message).map(|_| ()),
        }
    }

    fn ecrire(&mut self, message: &Value) -> Result<(), Erreur> {
        let Lien::Stdio { entree: Some(entree), .. } = &mut self.lien else { return Err(Erreur::Lien("entrée fermée".into())) };
        // serde_json n'écrit jamais de retour à la ligne : un message par ligne, comme le veut stdio.
        writeln!(entree, "{message}").and_then(|_| entree.flush()).map_err(|e| Erreur::Lien(format!("serveur arrêté ({e})")))
    }

    /// Lit les lignes jusqu'à la réponse `id`, au plus `delai` ; répond aux requêtes d'un serveur ancien (`ping`).
    fn attendre(&mut self, id: u64) -> Result<Value, Erreur> {
        let fin = Instant::now() + self.delai;
        loop {
            let Lien::Stdio { lignes, .. } = &self.lien else { unreachable!() };
            let ligne = match lignes.recv_timeout(fin.saturating_duration_since(Instant::now())) {
                Ok(l) => l,
                Err(RecvTimeoutError::Timeout) => return Err(Erreur::Lien(format!("aucune réponse en {} s", self.delai.as_secs_f64()))),
                Err(RecvTimeoutError::Disconnected) => return Err(Erreur::Lien("serveur arrêté".into())),
            };
            let Ok(message) = serde_json::from_str::<Value>(&ligne) else { continue };
            match (message.get("method"), message.get("id")) {
                (Some(methode), Some(sien)) => {
                    let reponse = if methode == "ping" {
                        json!({ "jsonrpc": "2.0", "id": sien, "result": {} })
                    } else {
                        json!({ "jsonrpc": "2.0", "id": sien, "error": { "code": -32601, "message": "méthode inconnue de tactile" } })
                    };
                    self.ecrire(&reponse)?;
                }
                (None, Some(sien)) if sien.as_u64() == Some(id) => return Ok(message),
                _ => {}
            }
        }
    }

    /// Un POST Streamable HTTP ; la réponse est un objet JSON ou un flux SSE dont on garde la réponse finale.
    fn poster(&mut self, message: &Value) -> Result<Option<Value>, Erreur> {
        let (methode, nom) = (message["method"].as_str().unwrap_or(""), message["params"]["name"].as_str());
        let Lien::Http { url, entetes, session, processus } = &mut self.lien else { unreachable!() };
        let mut tous = vec![("Content-Type".to_string(), "application/json".to_string()), ("Accept".into(), "application/json, text/event-stream".into())];
        match &self.ancienne {
            None => {
                tous.push(("MCP-Protocol-Version".into(), MODERNE.into()));
                tous.push(("Mcp-Method".into(), methode.into()));
                // shortcut: un nom d'outil hors ASCII n'est pas encodé en base64 (=?base64?…?=) ; le serveur le refusera.
                if let Some(nom) = nom.filter(|n| n.chars().all(|c| c.is_ascii_graphic())) {
                    tous.push(("Mcp-Name".into(), nom.into()));
                }
            }
            Some(version) if methode != "initialize" => tous.push(("MCP-Protocol-Version".into(), version.clone())),
            Some(_) => {}
        }
        if let Some(s) = session {
            tous.push(("Mcp-Session-Id".into(), s.clone()));
        }
        tous.extend(entetes.iter().cloned());
        let r = http::requete(url, &tous, Some(&message.to_string()), self.delai, processus).map_err(Erreur::Lien)?;
        if let Some(s) = r.entete("mcp-session-id").filter(|_| self.ancienne.is_some()) {
            *session = Some(s.to_string());
        }
        if r.statut == 202 || message.get("id").is_none() {
            return Ok(None);
        }
        let messages: Vec<Value> = if r.entete("content-type").is_some_and(|t| t.starts_with("text/event-stream")) {
            r.corps.split("\n\n").map(|evenement| {
                evenement.lines().filter_map(|l| l.strip_prefix("data:")).map(|d| d.strip_prefix(' ').unwrap_or(d)).collect::<Vec<_>>().join("\n")
            }).filter_map(|d| serde_json::from_str(&d).ok()).collect()
        } else {
            serde_json::from_str(&r.corps).into_iter().collect()
        };
        match messages.into_iter().find(|m| m.get("id") == message.get("id") || (r.statut >= 400 && m.get("error").is_some())) {
            Some(m) => Ok(Some(m)),
            None => Err(Erreur::Lien(format!("HTTP {}", r.statut))),
        }
    }
}

/// Fermer l'entrée (l'arrêt propre d'un serveur stdio), lui laisser une demi-seconde, puis tuer son groupe.
impl Drop for Client {
    fn drop(&mut self) {
        if let Lien::Stdio { enfant, entree, processus, .. } = &mut self.lien {
            drop(entree.take());
            let fin = Instant::now() + Duration::from_millis(500);
            while matches!(enfant.try_wait(), Ok(None)) && Instant::now() < fin {
                std::thread::sleep(Duration::from_millis(20));
            }
            let pid = enfant.id() as i32;
            tuer_groupe(pid);
            let _ = enfant.wait();
            retirer(processus, pid);
        }
    }
}

/// Le résultat d'un `tools/call` en valeur pour les gabarits.
fn resultat_outil(r: &Value) -> Result<Value, Erreur> {
    match r["resultType"].as_str() {
        None | Some("complete") => {}
        Some(autre) => return Err(Erreur::Rpc(0, format!("résultat « {autre} » non pris en charge"))),
    }
    let texte = r["content"][0]["text"].as_str();
    if r["isError"] == true {
        return Err(Erreur::Rpc(0, texte.unwrap_or("échec").chars().take(200).collect()));
    }
    match (r.get("structuredContent").filter(|s| !s.is_null()), texte) {
        (Some(s), _) => Ok(s.clone()),
        (None, Some(t)) => Ok(super::valeur_de_texte(t)),
        (None, None) => Err(Erreur::Rpc(0, "résultat sans texte".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVEUR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/exemples/mcp/horloge-mcp.py");

    fn processus() -> Processus {
        Processus::new(std::sync::Mutex::new(Some(Vec::new())))
    }

    #[test]
    fn resultats_d_outil() {
        assert_eq!(resultat_outil(&json!({ "content": [{ "type": "text", "text": "{\"heure\":\"12:00\"}" }] })).unwrap(), json!({ "heure": "12:00" }));
        assert_eq!(resultat_outil(&json!({ "content": [{ "type": "text", "text": "il pleut" }] })).unwrap(), json!({ "texte": "il pleut" }));
        assert_eq!(resultat_outil(&json!({ "content": [], "structuredContent": { "t": 3 }, "resultType": "complete" })).unwrap(), json!({ "t": 3 }));
        assert!(!resultat_outil(&json!({ "isError": true, "content": [{ "type": "text", "text": "ville inconnue" }] })).unwrap_err().relancer());
        assert!(resultat_outil(&json!({ "resultType": "input_required" })).is_err());
    }

    /// Serveur de démonstration, moderne puis ancien (`--ancien` : il ne connaît que `initialize`).
    #[test]
    fn serveur_stdio_des_deux_epoques() {
        for commande in [format!("python3 {SERVEUR}"), format!("python3 {SERVEUR} --ancien")] {
            let p = processus();
            let mut client = Client::stdio(&commande, &p, Duration::from_secs(10)).unwrap();
            assert_eq!(client.ancienne.is_some(), commande.ends_with("--ancien"), "{commande}");
            let v = client.appeler("heure", json!({})).unwrap();
            assert_eq!(v["heure"].as_str().map(str::len), Some(8), "{v}");
            // Un outil inconnu : erreur du serveur, qui reste vivant.
            assert!(!client.appeler("danser", json!({})).unwrap_err().relancer());
            assert!(client.appeler("heure", json!({})).is_ok());
            assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 1);
            drop(client);
            assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 0);
        }
    }

    #[test]
    fn serveur_tue_puis_relance() {
        let p = processus();
        let mut client = Client::stdio(&format!("python3 {SERVEUR}"), &p, Duration::from_secs(2)).unwrap();
        assert!(client.appeler("heure", json!({})).is_ok());
        let pid = p.lock().unwrap().as_ref().unwrap()[0];
        tuer_groupe(pid);
        let erreur = client.appeler("heure", json!({})).unwrap_err();
        assert!(erreur.relancer(), "{erreur}");
        drop(client);
        let mut client = Client::stdio(&format!("python3 {SERVEUR}"), &p, Duration::from_secs(2)).unwrap();
        assert!(client.appeler("heure", json!({})).is_ok());
        // La barre remplacée tue le serveur ; un nouveau lancement est refusé.
        crate::barre::tuer_tous(&p);
        assert!(client.appeler("heure", json!({})).unwrap_err().relancer());
        assert!(Client::stdio(&format!("python3 {SERVEUR}"), &p, Duration::from_secs(2)).is_err());
    }

    #[test]
    fn serveur_muet_et_commande_absente() {
        let p = processus();
        let debut = Instant::now();
        assert!(Client::stdio("sleep 39", &p, Duration::from_millis(300)).is_err());
        assert!(debut.elapsed() < Duration::from_secs(3));
        assert!(Client::stdio("commande-qui-nexiste-pas-tactile", &p, Duration::from_secs(2)).is_err());
        assert_eq!(p.lock().unwrap().as_ref().unwrap().len(), 0, "le sleep 39 est tué");
    }

    /// S3 : un serveur qui écrit 50 Mo sans fin de ligne. La ligne est coupée à 1 Mo et le serveur tué, sans que la
    /// mémoire du test prenne les 50 Mo (avant : `lines()` gardait toute la ligne).
    #[test]
    fn une_ligne_geante_arrete_le_serveur() {
        let p = processus();
        let avant = crate::essais::pic_memoire();
        let debut = Instant::now();
        assert!(Client::stdio("head -c 50000000 /dev/zero; sleep 21", &p, Duration::from_secs(30)).is_err());
        let pris = crate::essais::pic_memoire() - avant;
        assert!(debut.elapsed() < Duration::from_secs(10), "pas tué au dépassement : {:?}", debut.elapsed());
        assert!(pris < 30 << 20, "{} Mo de plus en mémoire", pris >> 20);
    }

    /// Streamable HTTP sur un serveur local : moderne en JSON, puis ancien (sonde refusée en 400, session, SSE).
    #[test]
    fn streamable_http() {
        let moderne = http::tests::serveur(|_, entetes, corps| {
            let m: Value = serde_json::from_str(corps).unwrap();
            assert!(entetes.contains("MCP-Protocol-Version: 2026-07-28"), "{entetes}");
            let result = match m["method"].as_str().unwrap() {
                "server/discover" => json!({ "supportedVersions": ["2026-07-28"], "capabilities": { "tools": {} } }),
                _ => {
                    assert!(entetes.contains("Mcp-Name: heure"), "{entetes}");
                    json!({ "content": [{ "type": "text", "text": "{\"heure\":\"08:30:00\"}" }] })
                }
            };
            (200, "application/json", json!({ "jsonrpc": "2.0", "id": m["id"], "result": result }).to_string())
        });
        let p = processus();
        let mut client = Client::http(&format!("{moderne}/mcp"), vec![], &p, Duration::from_secs(5)).unwrap();
        assert_eq!(client.appeler("heure", json!({})).unwrap(), json!({ "heure": "08:30:00" }));

        let ancien = http::tests::serveur(|_, entetes, corps| {
            let m: Value = serde_json::from_str(corps).unwrap();
            match m["method"].as_str().unwrap() {
                "server/discover" => (400, "application/json", json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32000, "message": "Bad Request: No valid session ID" } }).to_string()),
                "initialize" => (200, "text/event-stream", format!("event: message\ndata: {}\n\n", json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "protocolVersion": "2025-06-18" } }))),
                "notifications/initialized" => (202, "text/plain", String::new()),
                _ => {
                    assert!(entetes.contains("Mcp-Session-Id: s1") && entetes.contains("MCP-Protocol-Version: 2025-06-18"), "{entetes}");
                    (200, "text/event-stream", format!("data: {}\n\n", json!({ "jsonrpc": "2.0", "id": m["id"], "result": { "content": [{ "type": "text", "text": "midi" }] } })))
                }
            }
        });
        let mut client = Client::http(&format!("{ancien}/mcp"), vec![], &p, Duration::from_secs(5)).unwrap();
        assert_eq!(client.appeler("heure", json!({})).unwrap(), json!({ "texte": "midi" }));
        assert!(Client::http("http://example.org/mcp", vec![], &p, Duration::from_secs(1)).is_err());
    }
}
