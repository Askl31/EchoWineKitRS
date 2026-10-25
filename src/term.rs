//! Sortie terminal colorée (ANSI), désactivable via NO_COLOR.
//! Un « capture » optionnel permet à l'interface graphique de récupérer
//! toutes les lignes affichées (sans codes ANSI).

use std::io::IsTerminal;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy)]
pub enum Kind {
    Step,
    Ok,
    Warn,
    Err,
    Plain,
}

static COLOR: OnceLock<bool> = OnceLock::new();
static CAPTURE: Mutex<Option<Arc<Mutex<Vec<String>>>>> = Mutex::new(None);

fn color() -> bool {
    *COLOR.get_or_init(|| {
        std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
    })
}

/// Active la capture des lignes (interface graphique) et retourne le tampon partagé.
pub fn start_capture() -> Arc<Mutex<Vec<String>>> {
    let buf = Arc::new(Mutex::new(Vec::new()));
    *CAPTURE.lock().unwrap() = Some(buf.clone());
    buf
}

/// Désactive la capture (le tampon reste lisible par son détenteur).
pub fn stop_capture() {
    *CAPTURE.lock().unwrap() = None;
}

fn emit(kind: Kind, msg: &str) {
    let (prefix, code): (&str, &str) = match kind {
        Kind::Step => ("==> ", "1;34"),
        Kind::Ok => ("OK : ", "0;32"),
        Kind::Warn => ("ATTENTION : ", "1;33"),
        Kind::Err => ("ERREUR : ", "0;31"),
        Kind::Plain => ("", ""),
    };
    if let Some(cap) = CAPTURE.lock().unwrap().as_ref() {
        cap.lock().unwrap().push(format!("{prefix}{msg}"));
    }
    if !color() {
        match kind {
            Kind::Plain => println!("{msg}"),
            _ => println!("{prefix}{msg}"),
        }
        return;
    }
    match kind {
        Kind::Plain => println!("{msg}"),
        Kind::Step => println!("\x1b[{code}m{prefix}\x1b[0m \x1b[1m{msg}\x1b[0m"),
        _ => println!("\x1b[{code}m{prefix}{msg}\x1b[0m"),
    }
}

pub fn step(msg: &str) {
    emit(Kind::Step, msg);
}

pub fn ok(msg: &str) {
    emit(Kind::Ok, msg);
}

pub fn warn(msg: &str) {
    emit(Kind::Warn, msg);
}

pub fn err(msg: &str) {
    emit(Kind::Err, msg);
}

pub fn plain(msg: &str) {
    emit(Kind::Plain, msg);
}

/// Ligne de sortie brute d'une commande : indentation au terminal, texte intégral en capture.
pub fn raw_line(msg: &str) {
    if let Some(cap) = CAPTURE.lock().unwrap().as_ref() {
        cap.lock().unwrap().push(msg.to_string());
    }
    println!("    {msg}");
}

/// Ligne balisée ([OK]/[!!]/[??]) : colorée au terminal, balise conservée en capture.
pub fn tagged(tag: &str, code: &str, msg: &str) {
    if let Some(cap) = CAPTURE.lock().unwrap().as_ref() {
        cap.lock().unwrap().push(format!("  [{tag}] {msg}"));
        return;
    }
    if color() {
        println!("  \x1b[{code}m[{tag}]\x1b[0m {msg}");
    } else {
        println!("  [{tag}] {msg}");
    }
}
