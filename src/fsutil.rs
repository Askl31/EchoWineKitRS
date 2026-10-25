//! Copie récursive multi-thread avec progression, parcours borné en profondeur,
//! recherche de l'exécutable EchoPAC.

use crate::util::{human_bytes, human_rate, ProgressLine};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

struct Item {
    src: PathBuf,
    dst: PathBuf,
    size: u64,
}

pub struct CopyReport {
    pub files: u64,
    pub bytes: u64,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

/// Copie `src` → `dst` avec `workers` threads (défaut : cœurs disponibles, max 12).
/// Copie par blocs de 4 Mio (progression et débit réels, y compris sur setup.exe ~1 Gio).
pub fn copy_tree(src: &Path, dst: &Path) -> CopyReport {
    let mut report = CopyReport { files: 0, bytes: 0, skipped: Vec::new(), failed: Vec::new() };
    let mut items = Vec::new();
    let mut dirs = Vec::new();
    walk(src, dst, src, dst, &mut items, &mut dirs, &mut report);
    if !report.failed.is_empty() {
        return report;
    }
    // Dossiers d'abord, parents avant enfants (tri par profondeur).
    dirs.sort_by_key(|d| d.components().count());
    for d in &dirs {
        if let Err(e) = std::fs::create_dir_all(d) {
            report.failed.push(format!("{} : {e}", d.display()));
        }
    }
    let _ = std::fs::create_dir_all(dst);

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 12);
    let total: u64 = items.iter().map(|i| i.size).sum();
    let done = Arc::new(AtomicU64::new(0));
    let files_done = Arc::new(AtomicU64::new(0));
    let failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let (tx, rx) = mpsc::channel::<Item>();
    let rx = Arc::new(Mutex::new(rx));

    let stop = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let progress_done = done.clone();
    let progress_stop = stop.clone();
    let progress = std::thread::spawn(move || {
        let mut last_decile = 0u64;
        while !progress_stop.load(Ordering::Relaxed) {
            let d = progress_done.load(Ordering::Relaxed);
            let el = start.elapsed();
            ProgressLine::show(
                "copie",
                d,
                total,
                &format!(
                    "{}/{}  {}",
                    human_bytes(d),
                    human_bytes(total),
                    human_rate(d, el)
                ),
            );
            // jalons de 10 % : visibles au terminal ET dans l'interface graphique
            if total > 0 {
                let decile = d * 10 / total;
                if decile > last_decile {
                    last_decile = decile;
                    crate::term::plain(&format!(
                        "copie : {}0 % — {} / {}",
                        decile,
                        human_bytes(d),
                        human_bytes(total)
                    ));
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });

    let mut handles = Vec::new();
    for _ in 0..workers {
        let rx = rx.clone();
        let done = done.clone();
        let files_done = files_done.clone();
        let failures = failures.clone();
        handles.push(std::thread::spawn(move || loop {
            let item = { rx.lock().unwrap().recv() };
            let Ok(item) = item else { break };
            // copy_one incrémente `done` au fil des blocs écrits (progression réelle)
            if let Err(e) = copy_one(&item.src, &item.dst, &done) {
                failures.lock().unwrap().push(format!("{} : {e}", item.src.display()));
            }
            files_done.fetch_add(1, Ordering::Relaxed);
        }));
    }
    for item in items {
        let _ = tx.send(item);
    }
    drop(tx);
    for h in handles {
        let _ = h.join();
    }
    stop.store(true, Ordering::Relaxed);
    let _ = progress.join();
    ProgressLine::finish();

    let copied_bytes = done.load(Ordering::Relaxed);
    report.files = files_done.load(Ordering::Relaxed);
    report.bytes = copied_bytes;
    report.failed.extend(failures.lock().unwrap().drain(..));
    report
}

fn copy_one(src: &Path, dst: &Path, counter: &AtomicU64) -> std::io::Result<u64> {
    let mut r = std::fs::File::open(src)?;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut w = std::fs::File::create(dst)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = std::fs::metadata(src) {
            let _ = std::fs::set_permissions(dst, std::fs::Permissions::from_mode(m.permissions().mode()));
        }
    }
    let mut buf = vec![0u8; 4 * 1024 * 1024];
    let mut total = 0u64;
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        w.write_all(&buf[..n])?;
        total += n as u64;
        counter.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(total)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    root_src: &Path,
    root_dst: &Path,
    src: &Path,
    dst: &Path,
    items: &mut Vec<Item>,
    dirs: &mut Vec<PathBuf>,
    report: &mut CopyReport,
) {
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => {
            report.failed.push(format!("{} : {e}", src.display()));
            return;
        }
    };
    let _ = root_src;
    for entry in entries.flatten() {
        let path = entry.path();
        let child_dst = dst.join(entry.file_name());
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_dir() {
            dirs.push(child_dst.clone());
            walk(root_src, root_dst, &path, &child_dst, items, dirs, report);
        } else if ft.is_symlink() {
            report.skipped.push(format!("lien symbolique ignoré : {}", path.display()));
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            items.push(Item { src: path, dst: child_dst, size });
        }
    }
}

/// Parcours récursif borné : premier fichier « echopac*.exe » trouvé,
/// en excluant les désinstallateurs/installeurs (déterministe : tri alphabétique).
pub fn find_echopac_exe(drive_c: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    find_walk(drive_c, drive_c, 0, 5, &mut found);
    found.sort();
    found.into_iter().next()
}

fn find_walk(root: &Path, dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_dir() {
            find_walk(root, &path, depth + 1, max_depth, out);
        } else {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("echopac") && name.ends_with(".exe") && !name.contains("uninst") && !name.contains("setup") {
                out.push(path);
            }
        }
    }
}

/// Somme des tailles d'un arbre (pour doctor).
pub fn tree_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else { return 0 };
    for entry in entries.flatten() {
        let p = entry.path();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            total += tree_size(&p);
        } else {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

// ----- détection générique de médias d'installation EchoPAC -----

/// Dossier contenant setup.exe : la racine elle-même, sinon un sous-dossier
/// (≤ 2 niveaux, ex. « EchoPac 212.1.2.3 » dans ~/Downloads ou un volume monté).
pub fn find_setup_dir(root: &Path) -> Option<PathBuf> {
    if root.join("setup.exe").is_file() {
        return Some(root.to_path_buf());
    }
    let mut level: Vec<PathBuf> = vec![root.to_path_buf()];
    for _ in 0..2 {
        let mut next: Vec<PathBuf> = Vec::new();
        for dir in &level {
            let Ok(rd) = std::fs::read_dir(dir) else { continue };
            for e in rd.flatten() {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let p = e.path();
                    if p.join("setup.exe").is_file() {
                        return Some(p);
                    }
                    next.push(p);
                }
            }
            if next.len() > 200 {
                break; // garde-fou sur dossiers très peuplés
            }
        }
        level = next;
    }
    None
}

/// Premier fichier dont le nom (chemin complet inclus, casse ignorée) contient
/// `token` et dont l'extension correspond — parcours borné en profondeur.
pub fn find_by_token(root: &Path, token: &str, ext: &str, max_depth: usize) -> Option<PathBuf> {
    fn walk(dir: &Path, token: &str, ext: &str, depth: usize, max_depth: usize) -> Option<PathBuf> {
        if depth > max_depth {
            return None;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return None };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(hit) = walk(&p, token, ext, depth + 1, max_depth) {
                    return Some(hit);
                }
            } else {
                let name = p.to_string_lossy().to_lowercase();
                if name.contains(token) && name.ends_with(&format!(".{ext}")) {
                    return Some(p);
                }
            }
        }
        None
    }
    walk(root, token, ext, 0, max_depth)
}

/// Vrai si le dossier (ou un de ses sous-dossiers immédiats) contient setup.exe.
pub fn looks_like_media(root: &Path) -> bool {
    find_setup_dir(root).is_some()
}
