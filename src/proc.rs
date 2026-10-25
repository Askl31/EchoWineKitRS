//! Exécution de commandes : recherche sur PATH, sortie en direct + journal,
//! environnement Wine standard du kit.

use crate::config::Kit;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};

// ----- recherche de binaire (which) -----

pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|cand| is_executable_file(cand))
}

#[cfg(unix)]
fn is_executable_file(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(p) {
        Ok(m) => m.is_file() && (m.permissions().mode() & 0o111) != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

// ----- résolution Wine du kit -----

impl Kit {
    /// Binaire Wine : $WINE_BIN, puis `wine`, puis `wine-crossover` sur le PATH.
    pub fn wine_bin(&self) -> Option<PathBuf> {
        if let Some(p) = std::env::var_os("WINE_BIN") {
            let p = PathBuf::from(p);
            if p.is_file() {
                return Some(p);
            }
        }
        which("wine").or_else(|| which("wine-crossover"))
    }

    /// wineserver : PATH, puis frère du binaire Wine.
    pub fn wineserver_bin(&self) -> Option<PathBuf> {
        if let Some(p) = which("wineserver") {
            return Some(p);
        }
        self.wine_bin()
            .and_then(|w| w.parent().map(|d| d.join("wineserver")))
            .filter(|p| p.is_file())
    }
}

/// Commande Wine avec l'environnement standard du kit.
pub fn wine_cmd(kit: &Kit, debug: bool) -> Command {
    let mut c = Command::new(kit.wine_bin().expect("Wine introuvable (install-kit)"));
    c.env("WINEPREFIX", kit.prefix());
    c.env("WINEDEBUG", if debug { "warn+all" } else { "-all" });
    c.env("WINEDLLOVERRIDES", "mscoree,mshtml=");
    c
}

/// Commande winetricks ciblant le préfixe du kit.
pub fn winetricks_cmd(kit: &Kit) -> Command {
    let mut c = Command::new(which("winetricks").expect("winetricks introuvable (install-kit)"));
    c.env("WINEPREFIX", kit.prefix());
    c
}

// ----- exécution -----

/// Exécution interactive (stdio hérités) — pour les installateurs graphiques.
pub fn run_interactive(cmd: &mut Command) -> std::io::Result<ExitStatus> {
    cmd.status()
}

/// Exécution journalisée : sortie affichée en direct (optionnel) et ajoutée au log.
/// Retourne le statut, ou Err si le lancement échoue.
pub fn run_logged(
    cmd: &mut Command,
    log_path: &Path,
    live: bool,
) -> std::io::Result<ExitStatus> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = log_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let log = Arc::new(Mutex::new(
        OpenOptions::new().create(true).append(true).open(log_path)?,
    ));
    log_line(&log, &format!("$ {:?}", cmd));

    let mut child: Child = cmd.spawn()?;
    let out = child.stdout.take();
    let err = child.stderr.take();
    let h_out = out.map(|s| spawn_reader(s, log.clone(), live));
    let h_err = err.map(|s| spawn_reader(s, log, live));
    let status = child.wait();
    if let Some(h) = h_out {
        let _ = h.join();
    }
    if let Some(h) = h_err {
        let _ = h.join();
    }
    status
}

fn spawn_reader<R: Read + Send + 'static>(r: R, log: Arc<Mutex<File>>, live: bool) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(r);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim_end();
                    if live && !trimmed.is_empty() {
                        crate::term::raw_line(trimmed);
                    }
                    if let Ok(mut f) = log.lock() {
                        let _ = writeln!(f, "{trimmed}");
                    }
                }
            }
        }
    })
}

fn log_line(log: &Arc<Mutex<File>>, s: &str) {
    if let Ok(mut f) = log.lock() {
        let _ = writeln!(f, "{s}");
    }
}

/// Exécution silencieuse avec capture du stdout (pour les sondages type doctor).
pub fn capture(cmd: &mut Command) -> Option<String> {
    let out = cmd.output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}
