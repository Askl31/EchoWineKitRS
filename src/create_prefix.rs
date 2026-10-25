//! `create-prefix` — crée et configure le préfixe Wine dédié à EchoPAC.

use crate::config::Kit;
use crate::proc;
use crate::term;
use crate::util::timestamp;
use std::process::Command;

pub fn run(kit: &Kit) -> Result<(), String> {
    proc::which("winetricks").ok_or("winetricks absent — lancer d'abord `echowinekit install-kit`")?;
    kit.wine_bin().ok_or("Wine absent — lancer d'abord `echowinekit install-kit`")?;

    std::fs::create_dir_all(kit.logs()).map_err(|e| format!("création de {} impossible : {e}", kit.logs().display()))?;
    let log = kit.logs().join(format!("create-prefix-{}.log", timestamp()));

    // --- initialisation ---
    term::step(&format!("Préfixe Wine : {}", kit.prefix().display()));
    if kit.drive_c().exists() {
        term::warn("le préfixe existe déjà (pour le recréer : rm -rf du dossier prefix)");
    } else {
        std::fs::create_dir_all(kit.prefix()).map_err(|e| e.to_string())?;
        let mut boot = Command::new(kit.wine_bin().unwrap());
        boot.arg("wineboot").arg("-u")
            .env("WINEPREFIX", kit.prefix())
            .env("WINEARCH", "win64")
            .env("WINEDEBUG", "-all")
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=");
        let st = proc::run_logged(&mut boot, &log, true)
            .map_err(|e| format!("wineboot : {e}"))?;
        if st.success() {
            term::ok("préfixe initialisé (win64)");
        } else {
            return Err("l'initialisation du préfixe (wineboot) a échoué".into());
        }
    }

    // --- composants Windows ---
    term::step("Version Windows = Windows 10 + composants (peut prendre plusieurs minutes)");
    for (verb, note) in [
        ("win10", "réglage Windows 10"),
        ("corefonts", "polices standard"),
        ("vcrun2008", "VC++ 2008 (le média embarque aussi vs90_piaredist.exe)"),
        ("d3dcompiler_47", "compilateur de shaders D3D"),
    ] {
        term::plain(&format!("    winetricks {verb} — {note}"));
        let mut c = proc::winetricks_cmd(kit);
        c.args(["-q", verb]);
        let st = proc::run_logged(&mut c, &log, false).map_err(|e| format!("winetricks {verb} : {e}"))?;
        if st.success() {
            term::ok(&format!("{verb} installé"));
        } else {
            term::warn(&format!("{verb} : échec (journal : {})", log.display()));
        }
    }

    // --- DirectX 11 → D3DMetal (Metal) si disponible ---
    term::step("DirectX 11 : renderer D3DMetal (Metal) si disponible");
    let mut reg = proc::wine_cmd(kit, false);
    reg.arg("reg")
        .args(["add", r"HKCU\Software\Wine\Direct3D", "/v", "renderer", "/t", "REG_SZ", "/d", "d3dm", "/f"]);
    let d3dm_ok = proc::run_logged(&mut reg, &log, false)
        .map(|s| s.success())
        .unwrap_or(false);
    if d3dm_ok {
        term::ok("renderer d3dm activé (D3DMetal)");
    } else {
        term::warn("clé renderer non appliquée — WineD3D (OpenGL) sera utilisé");
    }

    // --- redémarrage propre ---
    term::step("Redémarrage du préfixe");
    if let Some(ws) = kit.wineserver_bin() {
        let _ = Command::new(ws).arg("-k").env("WINEPREFIX", kit.prefix()).status();
    }
    let mut boot = proc::wine_cmd(kit, false);
    boot.arg("wineboot").arg("-u");
    let _ = proc::run_logged(&mut boot, &log, false);

    term::ok(&format!("préfixe prêt. Journal : {}", log.display()));
    term::ok("Étape suivante : echowinekit install-echopac <chemin du média EchoPac>");
    Ok(())
}
