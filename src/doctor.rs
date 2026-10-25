//! `doctor` — diagnostic complet de la chaîne EchoWineKit.

use crate::config::Kit;
use crate::fsutil::{find_echopac_exe, tree_size};
use crate::proc;
use crate::term;
use crate::util::human_bytes;
use std::process::Command;

struct Report {
    ok: u32,
    ko: u32,
}

impl Report {
    fn pass(&mut self, msg: &str) {
        term::tagged("OK", "0;32", msg);
        self.ok += 1;
    }
    fn fail(&mut self, msg: &str) {
        term::tagged("!!", "0;31", msg);
        self.ko += 1;
    }
    fn maybe(&mut self, msg: &str) {
        term::tagged("??", "1;33", msg);
    }
}

pub fn run(kit: &Kit) -> Result<(), String> {
    let mut r = Report { ok: 0, ko: 0 };

    term::plain("=== EchoWineKit doctor ===");
    let swvers = proc::capture(Command::new("sw_vers").arg("-productVersion")).unwrap_or_default();
    term::plain(&format!("macOS    : {swvers} ({})", std::env::consts::ARCH));
    term::plain(&format!("kit      : {}", kit.root.display()));
    term::plain(&format!("préfixe  : {}", kit.prefix().display()));
    term::plain("");

    term::plain("[1] Briques système");
    if std::env::consts::ARCH == "aarch64" {
        let intel_ok = Command::new("arch")
            .args(["-x86_64", "/usr/bin/true"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if intel_ok {
            r.pass("Apple Silicon + Rosetta 2");
        } else {
            r.fail("Rosetta 2 absent (softwareupdate --install-rosetta)");
        }
    } else {
        r.maybe(&format!("architecture {} (toolkit pensé pour arm64)", std::env::consts::ARCH));
    }
    match proc::which("brew") {
        Some(b) => r.pass(&format!("Homebrew : {}", b.display())),
        None => r.fail("Homebrew absent (echowinekit install-kit)"),
    }
    term::plain("");

    term::plain("[2] Wine");
    match kit.wine_bin() {
        Some(w) => {
            r.pass(&format!("Wine : {}", w.display()));
            match proc::capture(Command::new(&w).arg("--version")) {
                Some(v) => term::plain(&format!("    version : {v}")),
                None => r.maybe("`wine --version` sans réponse"),
            }
        }
        None => r.fail("Wine absent (echowinekit install-kit)"),
    }
    term::plain("");

    term::plain("[3] Préfixe");
    if kit.drive_c().join("windows").is_dir() {
        r.pass("préfixe initialisé");
        term::plain(&format!("    taille du préfixe : {}", human_bytes(tree_size(&kit.prefix()))));
        for dll in ["d3d11.dll", "msvcp90.dll", "mfc90.dll"] {
            let found = find_file(&kit.drive_c(), dll, 0, 5);
            if found {
                r.pass(&format!("{dll} présent"));
            } else {
                r.maybe(&format!("{dll} non trouvé (peut être normal selon l'install)"));
            }
        }
    } else {
        r.fail("préfixe absent (echowinekit create-prefix)");
    }
    term::plain("");

    term::plain("[4] EchoPAC");
    let exe = kit.exe_unix().filter(|p| p.is_file()).or_else(|| find_echopac_exe(&kit.drive_c()));
    match exe {
        Some(e) => r.pass(&format!("exécutable : {}", e.display())),
        None => r.fail("EchoPAC non installé (echowinekit install-echopac <média>)"),
    }
    term::plain("");

    term::plain("[5] Direct3D");
    if kit.wine_bin().is_none() {
        r.maybe("Wine absent — vérification du renderer ignorée");
    } else {
        let mut q = proc::wine_cmd(kit, false);
        q.arg("reg").args(["query", r"HKCU\Software\Wine\Direct3D", "/v", "renderer"]);
        match proc::capture(&mut q) {
            Some(out) if out.contains("d3dm") => r.pass("renderer=d3dm (D3DMetal/Metal)"),
            Some(_) => r.maybe("renderer défini sans d3dm (WineD3D/OpenGL utilisé)"),
            None => r.maybe("clé renderer absente (WineD3D/OpenGL par défaut)"),
        }
    }
    term::plain("");

    term::plain("[6] Journaux récents");
    let mut logs: Vec<_> = std::fs::read_dir(kit.logs())
        .map(|rd| rd.flatten().collect())
        .unwrap_or_default();
    logs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    for i in 0..logs.len().min(5) {
        term::plain(&format!("    {}", logs[i].file_name().to_string_lossy()));
    }
    term::plain("");

    if r.ko == 0 {
        term::ok(&format!("{} points OK, aucun blocage détecté.", r.ok));
    } else {
        term::warn(&format!("{} OK, {} problème(s) [!!] — traiter le premier [!!] puis relancer.", r.ok, r.ko));
    }
    Ok(())
}

fn find_file(root: &std::path::Path, name: &str, depth: usize, max_depth: usize) -> bool {
    if depth > max_depth {
        return false;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return false };
    for e in rd.flatten() {
        let p = e.path();
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if find_file(&p, name, depth + 1, max_depth) {
                return true;
            }
        } else if e.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
            return true;
        }
    }
    false
}
