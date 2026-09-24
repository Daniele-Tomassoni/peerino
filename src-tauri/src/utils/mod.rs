// Peerino - P2P file sharing without cloud, without accounts, without intermediaries.
// Copyright (C) 2026 Daniele Tomassoni
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
pub mod network;

pub(crate) static ATOMIC_WRITE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Writes through a same-directory temporary file and renames atomically.
pub async fn atomic_write<F, Fut>(
    final_path: &std::path::Path,
    writer: F,
) -> Result<std::path::PathBuf, String>
where
    F: FnOnce(std::path::PathBuf) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let parent = final_path.parent().ok_or_else(|| "No parent directory".to_string())?;
    let tmp = parent.join(format!(".tmp_{}", uuid::Uuid::new_v4()));
    if let Err(e) = writer(tmp.clone()).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e);
    }
    let _rename_guard = ATOMIC_WRITE_LOCK.lock().await;
    let mut target = final_path.to_path_buf();
    let mut counter = 1;
    loop {
        if target.exists() {
            let stem = final_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = final_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_name = if ext.is_empty() { format!("{}_{}", stem, counter) } else { format!("{}_{}.{}", stem, counter, ext) };
            target = parent.join(new_name);
            counter += 1;
            continue;
        }
        match tokio::fs::rename(&tmp, &target).await {
            Ok(()) => return Ok(target),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists || target.exists() => {
                let stem = final_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
                let ext = final_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let new_name = if ext.is_empty() { format!("{}_{}", stem, counter) } else { format!("{}_{}.{}", stem, counter, ext) };
                target = parent.join(new_name);
                counter += 1;
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(format!("rename failed: {}", e));
            }
        }
    }
}
pub mod temp_cleanup;
pub mod peer_id;
pub mod turn_creds;
pub mod ice_provider;

/// Verifies that a filename is a single, ordinary filesystem component.
/// Rejects path traversal, Windows drive-relative/absolute syntax, reserved
/// device names, control characters, and names Windows would normalize.
pub fn is_safe_filename(filename: &str) -> bool {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
    {
        return false;
    }

    use std::path::{Component, Path};
    let mut components = Path::new(filename).components();
    let first = components.next();
    if components.next().is_some() || !matches!(first, Some(Component::Normal(_))) {
        return false;
    }

    let upper = filename.to_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6",
        "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
        "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem) {
        return false;
    }

    if filename.ends_with(' ') || filename.ends_with('.') {
        return false;
    }

    !filename.chars().any(char::is_control)
}

/// Returns the directory where the app executable is located.
/// This guarantees that the `shared-folder`, `temp`, and `config` folders
/// are created in the same directory as `peerino.exe`, regardless
/// of the current working directory.
pub fn get_app_dir() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

#[cfg(test)]
mod tests {
    use super::is_safe_filename;

    #[test]
    fn rejects_drive_relative_filename() {
        assert!(!is_safe_filename("C:evil.txt"));
    }

    #[test]
    fn rejects_windows_absolute_filename() {
        assert!(!is_safe_filename("C:\\evil.txt"));
    }

    #[test]
    fn rejects_parent_traversal_filename() {
        assert!(!is_safe_filename("..\\evil.txt"));
    }

    #[test]
    fn rejects_windows_reserved_device_name() {
        assert!(!is_safe_filename("CON"));
    }

    #[test]
    fn accepts_regular_filename() {
        assert!(is_safe_filename("file.txt"));
    }

    #[test]
    fn accepts_unicode_filename() {
        assert!(is_safe_filename("rapporté.txt"));
    }
}
