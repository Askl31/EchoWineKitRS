//! `uninstall-echopac` / `reset-kit` — désinstallation propre d'EchoPAC.
//!
//! Deux niveaux :
//! - uninstall-echopac : supprime le programme EchoPAC, les entrées de
//!   désinstallation du registre Wine, la copie du média (C:\media) et
//!   l'exécutable enregistré. Le préfixe et ses composants (Wine, VC++,
//!   SQL Anywhere) restent en place pour une réinstallation.
//! - reset-kit        : supprime tout le préfixe + la configuration
//!   (les journaux sont conservés). L'étape `create-prefix` devient
//!   nécessaire avant toute nouvelle installation.
//!
//! ⚠ Les données d'examen stockées DANS le préfixe sont supprimées :
//! l'interface prévient l'utilisateur avant de lancer ces tâches.

use crate::config::Kit;
use crate::fsutil::{find_echopac_exe, tree_size};
use crate::proc;
use crate::term;
use crate::util::human_bytes;
use std::path::Path;

const UNINSTALL_ROOTS: [&str; 3] = [
    r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
];

pub fn run(kit: &Kit) -> Result<(), String> {
    if !kit.drive_c().is_dir() {
        return Err("préfixe absent — rien à désinstaller".into());
    }

    // 1. Arrêter tout processus Wine résiduel
    term::step("Arrêt des processus Wine");
    if let Some(ws) = kit.wineserver_bin() {
        let _ = std::process::Command::new(ws)
            .arg("-k")
            .env("WINEPREFIX", kit.prefix())
            .status();
        term::ok("wineserver arrêté");
    } else {
        term::plain("    wineserver introuvable — poursuite");
    }

    // 2. Localiser l'installation
    let exe = find_echopac_exe(&kit.drive_c());
    let mut freed: u64 = 0;
    let install_dir = exe.as_ref().and_then(|e| e.parent().map(Path::to_path_buf));

    if let Some(dir) = &install_dir {
        // Garde-fou : ne jamais supprimer drive_c lui-même
        if dir == &kit.drive_c() || dir == &kit.prefix() {
            return Err("dossier d'installation inattendu (racine du préfixe) — désinstallation annulée".into());
        }
        freed += tree_size(dir);
    }

    // 3. Entrées de désinstallation du registre (best-effort, sans Wine installé : ignoré)
    term::step("Nettoyage des entrées de registre");
    if kit.wine_bin().is_some() {
        let mut cleaned = 0;
        for root in UNINSTALL_ROOTS {
            let mut q = proc::wine_cmd(kit, false);
            q.arg("reg").arg("query").arg(root);
            if let Some(out) = proc::capture(&mut q) {
                for line in out.lines() {
                    let key = line.trim();
                    if key.to_lowercase().contains("echopac") {
                        let mut d = proc::wine_cmd(kit, false);
                        d.arg("reg").arg("delete").arg(key).arg("/f");
                        if proc::capture(&mut d).is_some() {
                            cleaned += 1;
                        }
                    }
                }
            }
        }
        term::ok(&format!("{cleaned} entrée(s) de désinstallation supprimée(s)"));
    } else {
        term::plain("    Wine absent — nettoyage du registre ignoré");
    }

    // 4. Suppression du programme
    if let Some(dir) = &install_dir {
        term::step(&format!("Suppression de {}", dir.display()));
        match std::fs::remove_dir_all(dir) {
            Ok(()) => term::ok("programme supprimé"),
            Err(e) => term::warn(&format!("suppression partielle : {e}")),
        }
    } else {
        term::plain("    EchoPAC introuvable dans le préfixe (déjà désinstallé ?)");
    }

    // 5. Copie du média d'installation (C:\media)
    if kit.media_dest().exists() {
        freed += tree_size(&kit.media_dest());
        term::step("Suppression de la copie du média d'installation");
        match std::fs::remove_dir_all(kit.media_dest()) {
            Ok(()) => term::ok("média supprimé"),
            Err(e) => term::warn(&format!("suppression partielle : {e}")),
        }
    }

    // 6. Oublier l'exécutable enregistré
    kit.conf_set("ECHOPAC_EXE", "");

    term::ok(&format!(
        "désinstallation terminée — environ {} libérés (composants du préfixe conservés ; réinstaller avec l'étape 3)",
        human_bytes(freed)
    ));
    Ok(())
}

pub fn reset(kit: &Kit) -> Result<(), String> {
    // Arrêter Wine avant de supprimer le préfixe
    if let Some(ws) = kit.wineserver_bin() {
        let _ = std::process::Command::new(ws)
            .arg("-k")
            .env("WINEPREFIX", kit.prefix())
            .status();
    }
    if kit.prefix().exists() {
        let freed = tree_size(&kit.prefix());
        term::step(&format!("Suppression du préfixe {}", kit.prefix().display()));
        std::fs::remove_dir_all(kit.prefix())
            .map_err(|e| format!("suppression du préfixe impossible : {e}"))?;
        term::ok(&format!("préfixe supprimé ({} libérés)", human_bytes(freed)));
    } else {
        term::plain("préfixe absent — rien à supprimer");
    }
    if kit.conf_file().exists() {
        let _ = std::fs::remove_file(kit.conf_file());
        term::ok("configuration supprimée");
    }
    term::ok("réinitialisation terminée — relancer « Créer le préfixe » avant toute nouvelle installation");
    Ok(())
}
