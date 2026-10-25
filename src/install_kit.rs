//! `install-kit` — installe Rosetta 2, Homebrew, wine-crossover et winetricks.

use crate::proc;
use crate::term;
use std::process::Command;

pub fn run(kit: &crate::config::Kit) -> Result<(), String> {
    let _ = kit;

    if std::env::consts::OS != "macos" {
        return Err("Ce logiciel doit être exécuté sur macOS.".into());
    }

    // --- Rosetta 2 (nécessaire : wine-crossover est un binaire x86_64) ---
    term::step("Rosetta 2");
    let intel_ok = Command::new("arch")
        .args(["-x86_64", "/usr/bin/true"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if intel_ok {
        term::ok("déjà installé");
    } else if std::env::consts::ARCH == "aarch64" {
        term::plain("    Installation (mot de passe administrateur demandé)…");
        let st = Command::new("sudo")
            .args(["softwareupdate", "--install-rosetta", "--agree-to-license"])
            .status()
            .map_err(|e| format!("lancement de softwareupdate impossible : {e}"))?;
        if st.success() {
            term::ok("Rosetta 2 installé");
        } else {
            return Err("échec de l'installation de Rosetta 2".into());
        }
    } else {
        term::warn("Mac Intel détecté — Rosetta inutile, wine-crossover s'exécutera nativement.");
    }

    // --- Homebrew ---
    term::step("Homebrew");
    let brew = find_brew();
    let brew = match brew {
        Some(b) => {
            term::ok(&format!("{}", b.display()));
            b
        }
        None => {
            term::plain("    Installation (mot de passe demandé)…");
            let st = Command::new("/bin/bash")
                .arg("-c")
                .arg("curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | /bin/bash")
                .status()
                .map_err(|e| format!("lancement impossible : {e}"))?;
            if !st.success() {
                return Err("échec de l'installation de Homebrew".into());
            }
            find_brew().ok_or("Homebrew installé mais introuvable — rouvrir un terminal et relancer")?
        }
    };

    // --- wine-crossover (Wine + D3DMetal, gratuits) ---
    term::step("Wine « wine-crossover »");
    let already = proc::capture(Command::new(&brew).args(["list", "--cask"]))
        .map(|s| s.lines().any(|l| l.trim() == "wine-crossover"))
        .unwrap_or(false);
    if already {
        term::ok("déjà installé");
    } else {
        // --no-quarantine : évite le blocage Gatekeeper des binaires Wine
        let st = Command::new(&brew)
            .args(["install", "--cask", "--no-quarantine", "wine-crossover"])
            .status()
            .map_err(|e| format!("brew inaccessible : {e}"))?;
        if st.success() {
            term::ok("wine-crossover installé");
        } else {
            return Err("`brew install --cask wine-crossover` a échoué".into());
        }
    }

    // --- winetricks ---
    term::step("winetricks");
    if proc::which("winetricks").is_some() {
        term::ok("déjà présent");
    } else {
        let st = Command::new(&brew)
            .args(["install", "winetricks"])
            .status()
            .map_err(|e| format!("brew inaccessible : {e}"))?;
        if st.success() {
            term::ok("winetricks installé");
        } else {
            return Err("`brew install winetricks` a échoué".into());
        }
    }

    term::ok("Kit prêt. Étape suivante : echowinekit create-prefix");
    Ok(())
}

/// Trouve brew sur le PATH puis aux emplacements standard Apple Silicon / Intel.
fn find_brew() -> Option<std::path::PathBuf> {
    proc::which("brew").or_else(|| {
        ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
            .iter()
            .map(std::path::PathBuf::from)
            .find(|p| p.is_file())
    })
}
