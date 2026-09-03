use std::{
    collections::HashMap,
    fmt,
    io::{self, Cursor, Read},
    sync::{Arc, RwLock},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;
use zip::ZipArchive;

pub type FS = Arc<RwLock<HashMap<String, Vec<u8>>>>;
pub type ArchiveLoadStatus = Arc<RwLock<crate::ArchiveLoadStatus>>;

#[derive(Debug)]
pub enum FileError {
    FetchError(String),
    UnzipError(String),
    IoError(io::Error),
}

// 1. Implement Display to define how errors print to users
impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::FetchError(msg) => write!(f, "Fetch Error: {}", msg),
            FileError::UnzipError(msg) => write!(f, "Unzip Error: {}", msg),
            FileError::IoError(err) => write!(f, "IO Error: {}", err),
        }
    }
}

// 2. Implement the standard Error trait
impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

#[wasm_bindgen]
pub async fn fetch_file_from_server(url: &str) -> Result<Vec<u8>, JsValue> {
    let window = web_sys::window().ok_or("no global window found")?;

    // Call the browser's fetch() API
    let resp_value = JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: Response = resp_value.dyn_into()?;

    // 2. Extract the response body as a JavaScript ArrayBuffer
    let array_buffer_value = JsFuture::from(resp.array_buffer()?).await?;

    // 3. Convert the JS ArrayBuffer cleanly into a Rust Vec<u8>
    let type_array = js_sys::Uint8Array::new(&array_buffer_value);
    let zip_bytes: Vec<u8> = type_array.to_vec();

    Ok(zip_bytes)
}

// Your previous code modified slightly to yield standard errors if desired
pub async fn fetch_and_unzip(
    url: &str,
    status: &ArchiveLoadStatus,
) -> Result<HashMap<String, Vec<u8>>, FileError> {
    let mut file_system: HashMap<String, Vec<u8>> = HashMap::new();

    status.write().unwrap().phase = Some("Downloading archive...".to_string());
    let zip_bytes = fetch_file_from_server(url)
        .await
        .map_err(|e| FileError::FetchError(format!("{:?}", e)))?;

    status.write().unwrap().phase = Some("Opening archive...".to_string());
    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| FileError::UnzipError(format!("{:?}", e)))?;

    // Iterate through each file inside the ZIP
    for i in 0..archive.len() {
        status.write().unwrap().phase = Some(format!(
            "Extracting files... {}/{}",
            i + 1,
            archive.len()
        ));
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileError::UnzipError(format!("{:?}", e)))?;
        let name = file.name().to_string();

        if file.is_file() {
            // Read the file contents into a buffer or handle it as needed
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(FileError::IoError)?;

            file_system.insert(normalize_path(&name), contents);
        }
    }

    status.write().unwrap().phase = None;
    Ok(file_system)
}

pub fn find_file(
    file_system: &std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>>,
    requested_path: &str,
) -> Option<Vec<u8>> {
    let requested_path = normalize_path(requested_path);
    let file_system = file_system.read().ok()?;

    file_system
        .iter()
        .find_map(|(path, bytes)| (normalize_path(path) == requested_path).then(|| bytes.clone()))
}

pub fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_ascii_lowercase()
}
