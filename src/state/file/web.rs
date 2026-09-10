use std::{collections::HashMap, sync::{Arc, RwLock}};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, FormData, Request, RequestInit, Response, Url};

use super::{ArchiveLoadStatus, FileError};

const UPLOAD_URL: &str = "https://files.nif.cactus.vg/upload";

#[derive(serde::Deserialize)]
struct UploadResponse {
	url: String,
}

/// Fetches a URL through the browser and returns its response bytes.
#[wasm_bindgen]
pub async fn fetch_file_from_server(url: &str) -> Result<Vec<u8>, JsValue> {
	let window = web_sys::window().ok_or("no global window found")?;
	let response: Response = JsFuture::from(window.fetch_with_str(url)).await?.dyn_into()?;
	let array_buffer = JsFuture::from(response.array_buffer()?).await?;
	Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
}

/// Uploads an archive and returns the server-provided download URL.
pub async fn upload_file(
	file: File,
	status: &Arc<RwLock<crate::UploadStatus>>,
) -> Result<String, FileError> {
	status.write().unwrap().phase = Some(format!("Uploading {}...", file.name()));
	let form_data = FormData::new().map_err(fetch_error)?;
	form_data
		.append_with_blob_and_filename("file", &file, &file.name())
		.map_err(fetch_error)?;
	let request_init = RequestInit::new();
	request_init.set_method("POST");
	request_init.set_body(&form_data);
	let request = Request::new_with_str_and_init(UPLOAD_URL, &request_init).map_err(fetch_error)?;
	let window = web_sys::window().ok_or_else(|| FileError::FetchError("no global window found".into()))?;
	let response: Response = JsFuture::from(window.fetch_with_request(&request))
		.await
		.map_err(fetch_error)?
		.dyn_into()
		.map_err(fetch_error)?;
	if !response.ok() {
		return Err(FileError::FetchError(format!("Upload failed with HTTP status {}", response.status())));
	}
	let response_url = response.url();
	let response_text = JsFuture::from(response.text().map_err(fetch_error)?)
		.await
		.map_err(fetch_error)?
		.as_string()
		.ok_or_else(|| FileError::FetchError("Upload returned an empty response body".into()))?;
	let upload_response: UploadResponse = serde_json::from_str(&response_text)
		.map_err(|error| FileError::FetchError(format!("Invalid upload response: {error}")))?;
	let download_url = Url::new_with_base(&upload_response.url, &response_url)
		.map_err(fetch_error)?
		.href();
	status.write().unwrap().phase = None;
	Ok(download_url)
}

/// Downloads, extracts, and normalizes every file in a supported archive.
pub async fn fetch_and_extract(
	url: &str,
	status: &ArchiveLoadStatus,
) -> Result<HashMap<String, Vec<u8>>, FileError> {
	status.write().unwrap().phase = Some("Downloading archive...".to_string());
	let archive_bytes = fetch_file_from_server(url).await.map_err(fetch_error)?;
	super::extract_archive(archive_bytes, status)
}

fn fetch_error(error: JsValue) -> FileError {
	FileError::FetchError(format!("{error:?}"))
}
