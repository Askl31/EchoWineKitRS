//! Petits utilitaires : horodatage sans dépendance, tailles lisibles, barre de progression.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Horodatage local compact « AAAAMMJJ-HHMMSS » (UTC), sans dépendance externe.
pub fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Algorithme civil_from_days (H. Hinnant) : jours depuis l'epoch → (année, mois, jour).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 1536 MiB, 512.3 KiB…
pub fn human_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Kio", "Mio", "Gio", "Tio"];
    let mut v = b as f64;
    let mut i = 0usize;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} {}", UNITS[0])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Débit lisible à partir d'un Instant de départ.
pub fn human_rate(bytes: u64, elapsed: Duration) -> String {
    let s = elapsed.as_secs_f64();
    if s > 0.05 {
        format!("{}/s", human_bytes((bytes as f64 / s) as u64))
    } else {
        "…".into()
    }
}

/// Ligne de progression sur une seule ligne (retour chariot), terminée par `finish`.
pub struct ProgressLine;

impl ProgressLine {
    pub fn show(prefix: &str, done: u64, total: u64, suffix: &str) {
        let pct = if total == 0 { 100 } else { (done * 100 / total.max(1)).min(100) };
        const W: usize = 28;
        let filled = if total == 0 { W } else { (W * done as usize) / total.max(1) as usize };
        let bar: String = std::iter::repeat('=').take(filled)
            .chain(std::iter::repeat('.').take(W - filled))
            .collect();
        let line = format!("\r  {prefix} [{bar}] {pct:3}%  {suffix}   ");
        eprint!("{line}"); // stderr : non bufferisé, ne pollue pas stdout
    }

    pub fn finish() {
        eprintln!();
    }
}
