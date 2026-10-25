//! Chemins du kit et configuration persistante (fichier clé=valeur minimaliste).

use std::path::{Component, Path, PathBuf};

#[derive(Clone)]
pub struct Kit {
    pub root: PathBuf,
}

impl Kit {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn prefix(&self) -> PathBuf {
        self.root.join("prefix")
    }

    pub fn drive_c(&self) -> PathBuf {
        self.prefix().join("drive_c")
    }

    pub fn media_dest(&self) -> PathBuf {
        self.drive_c().join("media")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn conf_file(&self) -> PathBuf {
        self.root.join("echopac.conf")
    }

    // ----- configuration persistante (ECHOPAC_EXE=...) -----

    pub fn load_conf(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Ok(s) = std::fs::read_to_string(self.conf_file()) {
            for line in s.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    out.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
        out
    }

    pub fn conf_get(&self, key: &str) -> Option<String> {
        self.load_conf().into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn conf_set(&self, key: &str, value: &str) {
        let mut entries = self.load_conf();
        let file = self.conf_file();
        let _ = std::fs::create_dir_all(&self.root);
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.to_string(),
            None => entries.push((key.to_string(), value.to_string())),
        }
        let body: String = entries
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&file, body + "\n");
    }

    /// Chemin UNIX de l'exécutable EchoPAC enregistré (ou none).
    pub fn exe_unix(&self) -> Option<PathBuf> {
        let w = self.conf_get("ECHOPAC_EXE").filter(|v| !v.is_empty())?;
        to_unix(&self.drive_c(), &w)
    }
}

/// Chemin UNIX → chemin Windows (C:\\media\\setup.exe) pour les arguments de Wine.
pub fn to_windows(drive_c: &Path, unix: &Path) -> Option<String> {
    let rel = unix.strip_prefix(drive_c).ok()?;
    let mut parts: Vec<String> = vec!["C:".into()];
    for c in rel.components() {
        if let Component::Normal(s) = c {
            parts.push(s.to_string_lossy().into_owned());
        }
    }
    Some(parts.join("\\"))
}

/// Chemin Windows (C:\\...) → chemin UNIX dans drive_c.
pub fn to_unix(drive_c: &Path, win: &str) -> Option<PathBuf> {
    let rest = win
        .strip_prefix(|c: char| c.eq_ignore_ascii_case(&'c'))
        .unwrap_or(win)
        .strip_prefix(':')
        .unwrap_or(win);
    let mut p = drive_c.to_path_buf();
    for seg in rest.split(['\\', '/']) {
        if !seg.is_empty() {
            p.push(seg);
        }
    }
    Some(p)
}

/// Racine par défaut : $HOME/EchoPAC-WineKit
pub fn default_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("EchoPAC-WineKit")
}
