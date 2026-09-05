use std::{
    collections::HashMap,
    fmt,
    io::{self, Cursor, Read},
    sync::{Arc, RwLock},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, FormData, Request, RequestInit, Response, Url};
use zip::ZipArchive;

pub type FS = Arc<RwLock<HashMap<String, Vec<u8>>>>;
pub type ArchiveLoadStatus = Arc<RwLock<crate::ArchiveLoadStatus>>;

const UPLOAD_URL: &str = "https://files.nif.cactus.vg/upload";

#[derive(Debug)]
pub enum FileError {
    FetchError(String),
    UnzipError(String),
    IoError(io::Error),
}

#[derive(serde::Deserialize)]
struct UploadResponse {
    url: String,
}

impl fmt::Display for FileError {
    /// Formats an archive or network error for display in the viewer.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::FetchError(msg) => write!(f, "Fetch Error: {}", msg),
            FileError::UnzipError(msg) => write!(f, "Unzip Error: {}", msg),
            FileError::IoError(err) => write!(f, "IO Error: {}", err),
        }
    }
}

impl std::error::Error for FileError {
    /// Exposes the underlying I/O error when one is available.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

#[wasm_bindgen]
/// Fetches a URL through the browser and returns its response bytes.
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

/// Uploads an archive and returns the server-provided download URL.
pub async fn upload_file(
    file: File,
    status: &Arc<RwLock<crate::UploadStatus>>,
) -> Result<String, FileError> {
    status.write().unwrap().phase = Some(format!("Uploading {}...", file.name()));

    let form_data = FormData::new().map_err(|error| FileError::FetchError(format!("{error:?}")))?;
    form_data
        .append_with_blob_and_filename("file", &file, &file.name())
        .map_err(|error| FileError::FetchError(format!("{error:?}")))?;

    let request_init = RequestInit::new();
    request_init.set_method("POST");
    request_init.set_body(&form_data);
    let request = Request::new_with_str_and_init(UPLOAD_URL, &request_init)
        .map_err(|error| FileError::FetchError(format!("{error:?}")))?;
    let window = web_sys::window()
        .ok_or_else(|| FileError::FetchError("no global window found".to_string()))?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|error| FileError::FetchError(format!("{error:?}")))?
        .dyn_into()
        .map_err(|error| FileError::FetchError(format!("{error:?}")))?;

    if !response.ok() {
        return Err(FileError::FetchError(format!(
            "Upload failed with HTTP status {}",
            response.status()
        )));
    }

    let response_url = response.url();
    let response_text = JsFuture::from(
        response
            .text()
            .map_err(|error| FileError::FetchError(format!("{error:?}")))?,
    )
    .await
    .map_err(|error| FileError::FetchError(format!("{error:?}")))?
    .as_string()
    .ok_or_else(|| FileError::FetchError("Upload returned an empty response body".to_string()))?;
    let upload_response: UploadResponse = serde_json::from_str(&response_text)
        .map_err(|error| FileError::FetchError(format!("Invalid upload response: {error}")))?;
    let download_url = Url::new_with_base(&upload_response.url, &response_url)
        .map_err(|error| FileError::FetchError(format!("Invalid upload URL: {error:?}")))?
        .href();

    status.write().unwrap().phase = None;
    Ok(download_url)
}

/// Downloads, extracts, and normalizes every file in a ZIP archive.
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
        status.write().unwrap().phase =
            Some(format!("Extracting files... {}/{}", i + 1, archive.len()));
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

/// Finds an archive file by normalized, case-insensitive path.
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

/// Converts an archive path to the viewer's canonical backslash lowercase form.
pub fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_ascii_lowercase()
}
