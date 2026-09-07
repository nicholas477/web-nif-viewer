
use bevy::prelude::*;

use wasm_bindgen::JsCast;

const RECENT_FILES_COOKIE: &str = "esp_viewer_recent_files";

/// Reads persisted recent archive/file pairs from the browser cookie.
pub fn recent_files() -> crate::RecentFiles {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(cookies) = document.cookie() else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Some(encoded_value) = cookies
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{RECENT_FILES_COOKIE}=")))
    else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Ok(value) = js_sys::decode_uri_component(encoded_value) else {
        return crate::RecentFiles { files: Vec::new() };
    };
    let Some(value) = value.as_string() else {
        return crate::RecentFiles { files: Vec::new() };
    };

    crate::RecentFiles {
        files: value
            .lines()
            .filter_map(|entry| {
                let (zip_url, file_name) = entry.split_once('\t')?;
                (!zip_url.is_empty() && !file_name.is_empty()).then(|| crate::RecentFile {
                    zip_url: zip_url.to_string(),
                    file_name: file_name.to_string(),
                })
            })
            .collect(),
    }
}

/// Stores a successful archive/file selection at the front of the recent-files cookie.
pub fn record_recent_file(zip_url: &str, file_name: &str) {
    if zip_url.is_empty() {
        return;
    }
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };

    let mut files = recent_files();
    files.files.retain(|recent| recent.zip_url != zip_url);
    files.files.insert(
        0,
        crate::RecentFile {
            zip_url: zip_url.to_string(),
            file_name: file_name.to_string(),
        },
    );
    files.files.truncate(super::MAX_RECENT_FILES);

    let value = files
        .files
        .iter()
        .map(|recent| format!("{}\t{}", recent.zip_url, recent.file_name))
        .collect::<Vec<_>>()
        .join("\n");
    let encoded_value = js_sys::encode_uri_component(&value);
    let cookie =
        format!("{RECENT_FILES_COOKIE}={encoded_value}; Max-Age=31536000; Path=/; SameSite=Lax");
    if let Err(error) = document.set_cookie(&cookie) {
        bevy::log::warn!("Could not save recent files cookie: {:?}", error);
    }
}