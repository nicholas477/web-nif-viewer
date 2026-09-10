use std::{collections::HashMap, fs, path::Path};

use super::{ArchiveLoadStatus, FileError};

/// Reads and extracts a locally selected archive.
pub fn read_archive(
	path: &Path,
	status: &ArchiveLoadStatus,
) -> Result<HashMap<String, Vec<u8>>, FileError> {
	fs::read(path)
		.map_err(FileError::IoError)
		.and_then(|bytes| super::extract_archive(bytes, status))
}
