//! EchoWineKit — couche d'exécution EchoPAC dédiée (Wine + D3DMetal) pour macOS Apple Silicon.
//! Binaire unique, zéro dépendance : installation, préfixe, installation d'EchoPAC,
//! lancement, diagnostic, bundle /Applications.

mod app_bundle;
mod config;
mod create_prefix;
mod doctor;
mod fsutil;
mod gui;
mod install_echopac;
mod install_kit;
mod launch;
mod proc;
mod term;
mod uninstall;
mod util;

use config::Kit;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
EchoWineKit 1.0 — exécution d'EchoPAC sur macOS Apple Silicon (Wine + D3DMetal)

USAGE :
  echowinekit <commande> [options]

COMMANDES :
  install-kit                 Installe Rosetta 2, Homebrew, wine-crossover, winetricks
  create-prefix               Crée et configure le préfixe Wine dédié (win10, VC++2008,
                              D3D compiler, renderer D3DMetal)
  install-echopac <média>     Copie le média (multi-thread) et installe EchoPAC.
                              Accepte toute version d'EchoPAC (mise à jour incluse) :
                              le dossier contenant setup.exe est localisé automatiquement.
                              options : --debug (trace Wine maximale)
  launch                      Lance EchoPAC
  doctor                      Diagnostic complet de la chaîne
  app-bundle                  Crée /Applications/EchoPAC.app
  gui                         Interface graphique (navigateur, 127.0.0.1)
  uninstall-echopac           Désinstalle EchoPAC (programme + média copié + registre ;
                              préfixe conservé). Les données d'examen du préfixe sont supprimées.
  reset-kit                   Supprime le préfixe et la configuration (journaux conservés)

OPTIONS GLOBALES :
  --kit-root <chemin>         Racine du kit (défaut : $HOME/EchoPAC-WineKit)
  --help, -h                  Cette aide

ORDRE D'EMPLOI : install-kit → create-prefix → install-echopac → launch
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut root: Option<PathBuf> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kit-root" => {
                i += 1;
                match args.get(i) {
                    Some(v) => root = Some(PathBuf::from(v)),
                    None => {
                        term::err("--kit-root exige un chemin");
                        return ExitCode::FAILURE;
                    }
                }
            }
            "--help" | "-h" | "help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--version" => {
                println!("echowinekit 1.0.0");
                return ExitCode::SUCCESS;
            }
            other => rest.push(other.to_string()),
        }
        i += 1;
    }

    let kit = Kit::new(root.unwrap_or_else(config::default_root));
    let cmd = rest.first().cloned().unwrap_or_default();
    let result = match cmd.as_str() {
        "install-kit" => install_kit::run(&kit),
        "create-prefix" => create_prefix::run(&kit),
        "install-echopac" => {
            let mut debug = false;
            let media: Vec<&String> = rest[1..]
                .iter()
                .filter(|a| {
                    if a.as_str() == "--debug" {
                        debug = true;
                        false
                    } else {
                        true
                    }
                })
                .collect();
            match media.first() {
                Some(m) => install_echopac::run(&kit, &PathBuf::from(m.as_str()), debug),
                None => Err("usage : echowinekit install-echopac [--debug] <chemin du média>".into()),
            }
        }
        "launch" => launch::run(&kit),
        "doctor" => doctor::run(&kit),
        "app-bundle" => app_bundle::run(&kit),
        "uninstall-echopac" => uninstall::run(&kit),
        "reset-kit" => uninstall::reset(&kit),
        "gui" => gui::run(kit),
        "" => {
            println!("{USAGE}");
            return ExitCode::FAILURE;
        }
        unknown => Err(format!("commande inconnue : {unknown} (— --help pour l'aude)")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            term::err(&e);
            ExitCode::FAILURE
        }
    }
}
