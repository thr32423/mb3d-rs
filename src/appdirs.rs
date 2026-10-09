//! MB3D's working folders next to the program (`IniDirs` in
//! `FileHandling.pas`): `History` (the parameters of every calculated image),
//! `Meshes` (BulbTracer output) and `BigRenders` (tiled renders).

use std::path::PathBuf;

/// The folder of the executable (MB3D's `AppFolder`).
pub fn app_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// `name` next to the program, created if missing; falls back to the
/// working directory when the program's folder is not writable.
pub fn work_folder(name: &str) -> PathBuf {
    let d = app_folder().join(name);
    if std::fs::create_dir_all(&d).is_ok() {
        return d;
    }
    let d = std::env::current_dir().unwrap_or_default().join(name);
    let _ = std::fs::create_dir_all(&d);
    d
}

/// UTC date and time as (`yyyy-mm-dd`, `hhhmmmsss`), MB3D's history names.
pub fn history_stamp() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // civil date from days since 1970-01-01 (H. Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}h{:02}m{:02}s", rem / 3600, rem / 60 % 60, rem % 60))
}

/// `StoreHistoryPars`: writes the parameters of a calculated image to
/// `History/<date>/<time> <title>.m3p`; returns the file.
pub fn store_history(sc: &crate::scene::Scene, title: &str) -> Option<PathBuf> {
    let (date, time) = history_stamp();
    let dir = work_folder("History").join(date);
    std::fs::create_dir_all(&dir).ok()?;
    let t: String = title.chars().filter(|c| !"/\\:*?\"<>|".contains(*c)).collect();
    let name = if t.trim().is_empty() || t == "untitled" { format!("{time}.m3p") } else { format!("{time} {}.m3p", t.trim()) };
    let p = dir.join(name);
    std::fs::write(&p, crate::m3p::write(sc)).ok()?;
    Some(p)
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamp_format() {
        let (d, t) = super::history_stamp();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
        assert_eq!(t.len(), 9);
        assert_eq!(&t[2..3], "h");
    }
}
