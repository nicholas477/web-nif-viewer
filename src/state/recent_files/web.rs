
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
                let (source, zip_url, file_name) = match entry.splitn(3, '\t').collect::<Vec<_>>().as_slice() {
                    [source, zip_url, file_name] => (
                        match *source {
                            "disk" => crate::RecentFileSource::Disk,
                            _ => crate::RecentFileSource::Url,
                        },
                        *zip_url,
                        *file_name,
                    ),
                    [zip_url, file_name] => (crate::RecentFileSource::Url, *zip_url, *file_name),
                    _ => return None,
                };
                (!zip_url.is_empty() && !file_name.is_empty()).then(|| crate::RecentFile {
                    path: zip_url.to_string(),
                    file_name: file_name.to_string(),
                    source,
                })
            })
            .collect(),
    }
}

/// Stores a successful archive/file selection at the front of the recent-files cookie.
pub fn record_recent_file(
    source: crate::RecentFileSource,
    zip_url: &str,
    file_name: &str,
) {
    if zip_url.is_empty() {
        return;
    }
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(document) = document.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };

    let recent_file = crate::RecentFile {
        path: zip_url.to_string(),
        file_name: file_name.to_string(),
        source,
    };

    bevy::log::info!("Recording recent file: {recent_file:?}");

    let mut files = recent_files();
    files.files.retain(|recent| recent.path != zip_url);
    files.files.insert(
        0,
        recent_file,
    );
    files.files.truncate(super::MAX_RECENT_FILES);

    let value = files
        .files
        .iter()
        .map(|recent| {
            let source = match recent.source {
                crate::RecentFileSource::Disk => "disk",
                crate::RecentFileSource::Url => "url",
            };
            format!("{source}\t{}\t{}", recent.path, recent.file_name)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let encoded_value = js_sys::encode_uri_component(&value);
    let cookie =
        format!("{RECENT_FILES_COOKIE}={encoded_value}; Max-Age=31536000; Path=/; SameSite=Lax");
    if let Err(error) = document.set_cookie(&cookie) {
        bevy::log::warn!("Could not save recent files cookie: {:?}", error);
    }
}