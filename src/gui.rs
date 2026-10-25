//! `gui` — interface graphique légère : serveur HTTP local (127.0.0.1, port
//! éphémère) servi par le binaire lui-même, ouvert dans le navigateur par défaut.
//! Zéro dépendance : parsing HTTP minimal, JSON construit à la main, page
//! unique embarquée (gui.html). Les tâches s'exécutent en tâche de fond et
//! leur sortie est diffusée à la page par sondage.

use crate::config::Kit;
use crate::term;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

const HTML: &str = include_str!("gui.html");

struct Shared {
    running: Option<String>,
    ok: Option<bool>,
    buffer: Arc<Mutex<Vec<String>>>,
}

pub fn run(kit: Kit) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let url = format!("http://127.0.0.1:{port}");

    term::plain(&format!("EchoWineKit — interface disponible sur {url}"));
    term::plain("(laisser ce Terminal ouvert ; les demandes de mot de passe s'affichent ici)");
    if std::env::consts::OS == "macos" {
        let _ = std::process::Command::new("open").arg(&url).status();
    }

    let shared: Arc<Mutex<Shared>> = Arc::new(Mutex::new(Shared {
        running: None,
        ok: None,
        buffer: Arc::new(Mutex::new(Vec::new())),
    }));
    let busy = Arc::new(AtomicBool::new(false));

    for stream in listener.incoming().flatten() {
        let kit = kit.clone();
        let shared = shared.clone();
        let busy = busy.clone();
        thread::spawn(move || handle(stream, &kit, &shared, &busy));
    }
    Ok(())
}

// ----- requête / réponse HTTP minimales -----

/// Réponse d'erreur JSON (message + code HTTP).
fn json_err(msg: &str, code: &'static str) -> (&'static str, &'static str, Vec<u8>) {
    (
        code,
        "application/json",
        format!("{{\"error\":{}}}", json_str(msg)).into_bytes(),
    )
}

fn handle(mut stream: TcpStream, kit: &Kit, shared: &Arc<Mutex<Shared>>, busy: &Arc<AtomicBool>) {
    let Some((method, target, body)) = read_request(&mut stream) else {
        return;
    };
    let (route, query) = match target.split_once('?') {
        Some((r, q)) => (r.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };

    let (status, ctype, payload): (&str, &str, Vec<u8>) = match (method.as_str(), route.as_str()) {
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", HTML.as_bytes().to_vec()),
        ("GET", "/api/state") => ("200 OK", "application/json", state_json(kit).into_bytes()),
        ("GET", "/api/status") => ("200 OK", "application/json", status_json(shared).into_bytes()),
        ("GET", "/api/probe") => ("200 OK", "application/json", probe_json().into_bytes()),
        ("POST", "/api/start") => {
            let params = parse_form(&body);
            let step = params.get("step").cloned().unwrap_or_default();
            let media = params.get("media").filter(|m| !m.is_empty()).cloned();
            if busy.swap(true, Ordering::SeqCst) {
                json_err("une tâche est déjà en cours", "409 Conflict")
            } else if !matches!(
                step.as_str(),
                "install-kit"
                    | "create-prefix"
                    | "install-echopac"
                    | "launch"
                    | "doctor"
                    | "app-bundle"
                    | "uninstall-echopac"
                    | "reset-kit"
            ) {
                busy.store(false, Ordering::SeqCst);
                json_err("étape inconnue", "400 Bad Request")
            } else if step == "install-echopac" && media.is_none() {
                busy.store(false, Ordering::SeqCst);
                json_err("chemin du média requis (dossier contenant setup.exe)", "400 Bad Request")
            } else {
                let kit = kit.clone();
                let shared = shared.clone();
                let busy = busy.clone();
                thread::spawn(move || run_job(&kit, &shared, &busy, &step, media));
                ("200 OK", "application/json", b"{\"started\":true}".to_vec())
            }
        }
        ("POST", "/api/open") => {
            let params = parse_form(&body);
            let target_dir = match params.get("target").map(|s| s.as_str()) {
                Some("root") => Some(kit.root.clone()),
                Some("prefix") => Some(kit.prefix()),
                Some("logs") => Some(kit.logs()),
                Some("media") => Some(kit.media_dest()),
                _ => None,
            };
            match target_dir {
                Some(dir) => {
                    let _ = std::fs::create_dir_all(&dir);
                    let cmd = if std::env::consts::OS == "macos" {
                        std::process::Command::new("open").arg(&dir).status()
                    } else if std::env::consts::OS == "windows" {
                        std::process::Command::new("explorer").arg(&dir).status()
                    } else {
                        std::process::Command::new("xdg-open").arg(&dir).status()
                    };
                    let opened = cmd.map(|s| s.success()).unwrap_or(false);
                    if opened {
                        ("200 OK", "application/json", b"{\"opened\":true}".to_vec())
                    } else {
                        json_err("ouverture impossible", "500 Internal Server Error")
                    }
                }
                None => json_err("cible inconnue", "400 Bad Request"),
            }
        }
        _ => ("404 Not Found", "text/plain; charset=utf-8", b"introuvable".to_vec()),
    };
    let _ = query; // (aucun paramètre GET utilisé)

    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&payload);
    let _ = stream.flush();
}

fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            return None;
        }
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let mut len = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body_bytes = buf[header_end + 4..].to_vec();
    while body_bytes.len() < len {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        body_bytes.extend_from_slice(&tmp[..n]);
    }
    body_bytes.truncate(len);
    Some((method, target, String::from_utf8_lossy(&body_bytes).into_owned()))
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn parse_form(body: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pair in body.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(url_decode(k), url_decode(v));
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ----- exécution des tâches -----

fn run_job(kit: &Kit, shared: &Arc<Mutex<Shared>>, busy: &Arc<AtomicBool>, step: &str, media: Option<String>) {
    {
        let mut sh = shared.lock().unwrap();
        sh.running = Some(step.to_string());
        sh.ok = None;
        sh.buffer = term::start_capture();
    }
    term::plain(&format!("── tâche : {step} ──"));

    let result: Result<(), String> = match step {
        "install-kit" => crate::install_kit::run(kit),
        "create-prefix" => crate::create_prefix::run(kit),
        "install-echopac" => crate::install_echopac::run(kit, &PathBuf::from(media.unwrap_or_default()), false),
        "launch" => crate::launch::run(kit),
        "doctor" => crate::doctor::run(kit),
        "app-bundle" => crate::app_bundle::run(kit),
        "uninstall-echopac" => crate::uninstall::run(kit),
        "reset-kit" => crate::uninstall::reset(kit),
        _ => Err("étape inconnue".into()),
    };
    match &result {
        Ok(()) => term::ok("tâche terminée avec succès"),
        Err(e) => term::err(&format!("échec : {e}")),
    }
    term::stop_capture();

    {
        let mut sh = shared.lock().unwrap();
        sh.running = None;
        sh.ok = Some(result.is_ok());
    }
    busy.store(false, Ordering::SeqCst);
}

// ----- JSON -----

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn state_json(kit: &Kit) -> String {
    let kit_ok = crate::proc::which("brew").is_some() && kit.wine_bin().is_some();
    let prefix_ok = kit.drive_c().join("windows").is_dir();
    let exe_ok = kit.exe_unix().map(|p| p.is_file()).unwrap_or(false)
        || crate::fsutil::find_echopac_exe(&kit.drive_c()).is_some();
    format!(
        "{{\"kit\":{kit_ok},\"prefix\":{prefix_ok},\"echopac\":{exe_ok},\"root\":{},\"prefix_path\":{},\"logs_path\":{}}}",
        json_str(&kit.root.display().to_string()),
        json_str(&kit.prefix().display().to_string()),
        json_str(&kit.logs().display().to_string())
    )
}

fn status_json(shared: &Arc<Mutex<Shared>>) -> String {
    let sh = shared.lock().unwrap();
    let lines: Vec<String> = sh
        .buffer
        .lock()
        .unwrap()
        .iter()
        .map(|l| json_str(l))
        .collect();
    let running: String = sh.running.as_deref().map(json_str).unwrap_or_else(|| "null".into());
    let ok: String = match sh.ok {
        Some(true) => "true".into(),
        Some(false) => "false".into(),
        None => "null".into(),
    };
    format!(
        "{{\"running\":{running},\"ok\":{ok},\"lines\":[{}]}}",
        lines.join(",")
    )
}

fn probe_json() -> String {
    let mut cands: Vec<String> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(h) = std::env::var_os("HOME") {
        let h = PathBuf::from(h);
        for sub in ["Downloads", "Desktop", "Documents"] {
            roots.push(h.join(sub));
        }
    }
    if let Ok(rd) = std::fs::read_dir("/Volumes") {
        roots.extend(rd.flatten().map(|e| e.path()));
    }
    roots.push(PathBuf::from("."));
    'roots: for root in roots.iter().take(24) {
        let mut level: Vec<PathBuf> = vec![root.clone()];
        for _ in 0..2 {
            let mut next: Vec<PathBuf> = Vec::new();
            for dir in &level {
                if crate::fsutil::looks_like_media(dir) {
                    cands.push(json_str(&dir.display().to_string()));
                    if cands.len() >= 12 {
                        break 'roots;
                    }
                }
                if let Ok(rd) = std::fs::read_dir(dir) {
                    for e in rd.flatten() {
                        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            next.push(e.path());
                        }
                    }
                }
            }
            level = next;
        }
    }
    format!("{{\"candidates\":[{}]}}", cands.join(","))
}
