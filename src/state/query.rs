
use wasm_bindgen::JsValue;

/// Represents the state of the query parameters in the browser URL.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct QueryState {
    #[serde(rename = "zip")]
    pub zip_url: String,
    #[serde(rename = "file", skip_serializing_if = "Option::is_none")]
    pub selected_file: Option<String>,
}

/// Synchronizes the browser URL query parameters with archive and file selection state.
pub fn update_query(zip_url: &str, selected_file: Option<&str>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(current_url) = window.location().href() else {
        return;
    };
    let Ok(url) = web_sys::Url::new(&current_url) else {
        return;
    };
    let state = QueryState {
        zip_url: zip_url.to_string(),
        selected_file: selected_file.map(str::to_string),
    };
    let Ok(query) = serde_urlencoded::to_string(state) else {
        return;
    };
    url.set_search(&format!("?{query}"));
    if let Ok(history) = window.history()
        && let Err(error) = history.replace_state_with_url(&JsValue::NULL, "", Some(&url.href()))
    {
        bevy::log::warn!("Could not update page URL: {:?}", error);
    }
}

/// Reads the archive URL and selected file from the browser query string.
pub fn query_state() -> Option<QueryState> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    serde_urlencoded::from_str(search.strip_prefix('?').unwrap_or(&search)).ok()
}