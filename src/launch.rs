//! `launch` — démarre EchoPAC dans le préfixe dédié (environnement propre).

use crate::config::{to_windows, Kit};
use crate::install_echopac::detect_and_save;
use crate::proc;
use crate::term;
use std::process::Command;

pub fn run(kit: &Kit) -> Result<(), String> {
    if !kit.drive_c().exists() {
        return Err("préfixe absent — lancer d'abord `echowinekit create-prefix`".into());
    }
    kit.wine_bin().ok_or("Wine absent — lancer d'abord `echowinekit install-kit`")?;

    // Exécutable : config, sinon détection + enregistrement
    let exe_unix = match kit.exe_unix() {
        Some(p) if p.is_file() => p,
        _ => detect_and_save(kit).ok_or("EchoPAC.exe introuvable — lancer d'abord `echowinekit install-echopac <média>`")?,
    };
    let exe_win = to_windows(&kit.drive_c(), &exe_unix)
        .ok_or("chemin de l'exécutable non convertible")?;

    // État propre : arrêter un éventuel wineserver résiduel
    if let Some(ws) = kit.wineserver_bin() {
        let _ = Command::new(ws).arg("-k").env("WINEPREFIX", kit.prefix()).status();
    }

    term::ok(&format!("lancement : {exe_win}"));
    let mut c = proc::wine_cmd(kit, false);
    c.arg(&exe_win);
    let st = proc::run_interactive(&mut c).map_err(|e| format!("lancement de Wine : {e}"))?;
    match st.code() {
        Some(0) => Ok(()),
        Some(code) => {
            term::warn(&format!("EchoPAC s'est arrêté avec le code {code}"));
            term::plain("    Si un message exige une « clé » / « Sentinel » : cette voie suppose");
            term::plain("    que la clé n'est pas nécessaire — sinon voir la Solution 1 (VM + clé USB).");
            Ok(())
        }
        None => Ok(()), // interrompu par signal
    }
}
