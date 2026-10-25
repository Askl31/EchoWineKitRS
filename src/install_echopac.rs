//! `install-echopac` — copie le média (multi-thread) puis installe EchoPAC dans le préfixe.
//! Usage : echowinekit install-echopac [--debug] <chemin du média>
//!
//! GÉNÉRIQUE : accepte n'importe quel média d'installation EchoPAC (toute version,
//! mise à jour incluse). La seule exigence est la présence d'un setup.exe.
//! Le dossier contenant setup.exe est localisé automatiquement (racine ou
//! sous-dossier ≤ 2 niveaux, ex. « EchoPac 212.x.x.x » dans ~/Downloads),
//! de même que le MSI SQL Anywhere et le redistribuable VC++ s'ils existent.

use crate::config::{to_windows, Kit};
use crate::fsutil;
use crate::proc;
use crate::term;
use crate::util::{human_bytes, timestamp};
use std::path::{Path, PathBuf};

pub fn run(kit: &Kit, media_in: &Path, debug: bool) -> Result<(), String> {
    if !media_in.is_dir() {
        return Err(format!("dossier média introuvable : {}", media_in.display()));
    }
    if !kit.drive_c().exists() {
        return Err("préfixe absent — lancer d'abord `echowinekit create-prefix`".into());
    }
    kit.wine_bin().ok_or("Wine absent — lancer d'abord `echowinekit install-kit`")?;

    // --- localisation générique du média (dossier contenant setup.exe) ---
    let media = fsutil::find_setup_dir(media_in).ok_or_else(|| {
        format!(
            "aucun setup.exe trouvé dans {} (ni à la racine, ni jusqu'à 2 sous-dossiers)",
            media_in.display()
        )
    })?;
    term::ok(&format!("média détecté : {}", media.display()));

    let log = kit.logs().join(format!("install-echopac-{}.log", timestamp()));
    std::fs::create_dir_all(kit.logs()).ok();

    // --- 1/3 copie parallèle du média ---
    term::step(&format!("Copie du média vers {}", kit.media_dest().display()));
    if kit.media_dest().exists() {
        std::fs::remove_dir_all(kit.media_dest())
            .map_err(|e| format!("nettoyage de l'ancienne copie impossible : {e}"))?;
    }
    let report = fsutil::copy_tree(&media, &kit.media_dest());
    if !report.failed.is_empty() {
        for f in &report.failed {
            term::err(f);
        }
        return Err(format!("{} fichier(s) non copié(s)", report.failed.len()));
    }
    term::ok(&format!("{} fichiers copiés ({})", report.files, human_bytes(report.bytes)));
    for s in &report.skipped {
        term::warn(s);
    }

    // --- 2/3 composants annexes détectés sur le média (facultatifs) ---
    term::step("Composants annexes du média (VC++ redist, SQL Anywhere)");
    if let Some(vc) = fsutil::find_by_token(&kit.media_dest(), "piaredist", "exe", 5) {
        let wexe = to_windows(&kit.drive_c(), &vc).unwrap();
        let mut c = proc::wine_cmd(kit, debug);
        c.arg(&wexe).arg("/q");
        let st = proc::run_logged(&mut c, &log, false).map_err(|e| e.to_string())?;
        if st.success() {
            term::ok("VC++ redist installé");
        } else {
            term::warn("VC++ redist : code retour non nul (déjà présent ? on continue)");
        }
    } else {
        term::plain("    aucun redistribuable VC++ détecté — ignoré");
    }

    if let Some(msi) = fsutil::find_by_token(&kit.media_dest(), "sqlany", "msi", 5) {
        let wmsi = to_windows(&kit.drive_c(), &msi).unwrap();
        let mut c = proc::wine_cmd(kit, debug);
        c.arg("msiexec").arg("/i").arg(&wmsi).arg("/qb-");
        let st = proc::run_logged(&mut c, &log, false).map_err(|e| e.to_string())?;
        if st.success() {
            term::ok(&format!("SQL Anywhere installé ({})", msi.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()));
        } else {
            term::warn(&format!("MSI SQL Anywhere : échec ou partiel (journal : {})", log.display()));
        }
    } else {
        term::warn("aucun MSI SQL Anywhere détecté — l'installeur principal l'installera peut-être lui-même");
    }

    // --- 3/3 EchoPAC (setup.exe interactif) ---
    term::step("EchoPAC (setup.exe, InstallShield)");
    term::plain("    ⚠ Répondre « Non / Ignorer » si l'installeur propose d'installer un driver Sentinel/HASP");
    term::plain("    (l'installeur s'ouvre dans sa propre fenêtre ; une mise à jour s'installe par-dessus l'existante)");
    let mut c = proc::wine_cmd(kit, debug);
    c.arg(r"C:\media\setup.exe").current_dir(kit.media_dest());
    let st = proc::run_interactive(&mut c).map_err(|e| format!("setup.exe : {e}"))?;
    if !st.success() {
        term::warn(&format!(
            "setup.exe : code de sortie non nul ({}) — consulter le journal",
            st.code().unwrap_or(-1)
        ));
    }

    // --- exécutable installé ? ---
    term::step("Recherche de l'exécutable EchoPAC installé");
    match fsutil::find_echopac_exe(&kit.drive_c()) {
        Some(exe) => {
            let wexe = to_windows(&kit.drive_c(), &exe).unwrap_or_else(|| "C:\\EchoPAC.exe".into());
            kit.conf_set("ECHOPAC_EXE", &wexe);
            term::ok(&format!("EchoPAC installé : {wexe}"));
            term::ok("Lancer avec : echowinekit launch");
        }
        None => {
            term::warn("EchoPAC*.exe non trouvé dans le préfixe.");
            term::plain("  - vérifier que l'installeur s'est déroulé jusqu'au bout ;");
            term::plain(&format!("  - journal complet : {}", log.display()));
            term::plain("  - relancer avec --debug pour la trace maximale ;");
            term::plain("  - si l'installeur bloque sur Sentinel/HASP : cette voie suppose");
            term::plain("    que la clé n'est pas exigée (voir README §Limites).");
        }
    }
    Ok(())
}

/// Utilisé par `launch` quand rien n'est encore enregistré.
pub fn detect_and_save(kit: &Kit) -> Option<PathBuf> {
    let exe = fsutil::find_echopac_exe(&kit.drive_c())?;
    if let Some(w) = to_windows(&kit.drive_c(), &exe) {
        kit.conf_set("ECHOPAC_EXE", &w);
    }
    Some(exe)
}
