//! `app-bundle` — crée /Applications/EchoPAC.app pointant vers ce binaire.

use crate::term;
use std::path::PathBuf;

pub fn run(_kit: &crate::config::Kit) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("binaire courant introuvable : {e}"))?;
    let app = PathBuf::from("/Applications/EchoPAC.app");

    let macos = app.join("Contents/MacOS");
    std::fs::create_dir_all(&macos).map_err(|e| format!("création de {} impossible : {e}", macos.display()))?;
    std::fs::create_dir_all(app.join("Contents/Resources")).ok();

    let launcher = format!(
        "#!/bin/bash\n# Généré par EchoWineKit — relance le binaire en mode launch\nexec \"{}\" launch\n",
        exe.display()
    );
    std::fs::write(macos.join("EchoPAC"), launcher).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(macos.join("EchoPAC"), std::fs::Permissions::from_mode(0o755));
    }

    let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>    <string>EchoPAC</string>
    <key>CFBundleIdentifier</key>    <string>local.echopac.winekit</string>
    <key>CFBundleName</key>          <string>EchoPAC</string>
    <key>CFBundlePackageType</key>   <string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>LSMinimumSystemVersion</key><string>12.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#;
    std::fs::write(app.join("Contents/Info.plist"), plist).map_err(|e| e.to_string())?;

    term::ok(&format!("EchoPAC.app créé : {}", app.display()));
    term::plain(&format!("    il lance « {} launch »", exe.display()));
    term::plain("    icône optionnelle : remplacer EchoPAC.app/Contents/Resources/AppIcon.icns");
    Ok(())
}
