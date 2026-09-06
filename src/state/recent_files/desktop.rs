use std::fs;

use bevy::prelude::*;
use directories::ProjectDirs;

fn recent_files_path() -> Option<std::path::PathBuf> {
    ProjectDirs::from("com", "nicholas477", "nif-viewer")
        .map(|proj_dirs| proj_dirs.data_local_dir().join("recent_files.toml"))
        .or_else(|| {
            bevy::log::warn!("Could not determine project directories for recent files.");
            None
        })
}

/// Records a recent archive/file pair in the local configuration.
pub fn record_recent_file(archive_path: &str, file_name: &str) {
    let mut recent_files = recent_files();
    recent_files.files.retain(|recent| recent.zip_url != archive_path);
    recent_files.files.insert(
        0,
        crate::RecentFile {
            zip_url: archive_path.to_string(),
            file_name: file_name.to_string(),
        },
    );
    recent_files.files.truncate(super::MAX_RECENT_FILES);

    if let Some(recent_files_path) = recent_files_path()
        && let Ok(contents) = toml::to_string(&recent_files).inspect_err(|e| {
            bevy::log::warn!("Failed to serialize recent files to TOML: {e}");
        })
    {
        let _ = fs::create_dir_all(recent_files_path.parent().unwrap());
        let _ = fs::write(&recent_files_path, contents);
    }
}

/// Returns a list of recent files from the recent files toml configuration.
/// Win: C:\Users\Alice\AppData\Roaming\nicholas477\nif-viewer\data
pub fn recent_files() -> crate::RecentFiles {
    if let Some(recent_files_path) = recent_files_path()
        && recent_files_path.exists()
        && let Ok(contents) = fs::read_to_string(&recent_files_path)
        && let Ok(recent_files) =
            toml::from_str::<crate::RecentFiles>(&contents).inspect_err(|e| {
                bevy::log::warn!("Failed to deserialize recent files to TOML: {e}");
            })
    {
        return recent_files;
    }

    crate::RecentFiles { files: Vec::new() }
}