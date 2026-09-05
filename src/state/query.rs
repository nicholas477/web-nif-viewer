use wasm_bindgen::JsValue;

/// Represents the state of the query parameters in the browser URL.
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct QueryState {
    pub zip_url: String,
    pub selected_file: String,
    #[serde(with = "view_state_bytes")]
    pub view_state: crate::state::ViewState,
}

mod view_state_bytes {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use serde::{Deserialize, Deserializer, Serializer};

    const VERSION: u8 = 1;

    pub fn serialize<S>(
        view_state: &crate::state::ViewState,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = [
            VERSION,
            shading_mode_byte(view_state.shading_mode),
            display_mode_byte(view_state.vertex_colors),
            display_mode_byte(view_state.collision),
            u8::from(view_state.wireframe),
        ];
        serializer.serialize_str(&URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<crate::state::ViewState, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(serde::de::Error::custom)?;
        let [version, shading_mode, vertex_colors, collision, wireframe] = bytes.as_slice() else {
            return Err(serde::de::Error::custom("invalid view state byte length"));
        };
        if *version != VERSION {
            return Err(serde::de::Error::custom("unsupported view state version"));
        }

        Ok(crate::state::ViewState {
            shading_mode: shading_mode_from_byte(*shading_mode)
                .ok_or_else(|| serde::de::Error::custom("invalid shading mode"))?,
            vertex_colors: display_mode_from_byte(*vertex_colors)
                .ok_or_else(|| serde::de::Error::custom("invalid vertex color mode"))?,
            collision: display_mode_from_byte(*collision)
                .ok_or_else(|| serde::de::Error::custom("invalid collision mode"))?,
            wireframe: match wireframe {
                0 => false,
                1 => true,
                _ => return Err(serde::de::Error::custom("invalid wireframe value")),
            },
        })
    }

    fn shading_mode_byte(mode: crate::state::ShadingMode) -> u8 {
        match mode {
            crate::state::ShadingMode::Lit => 0,
            crate::state::ShadingMode::Unlit => 1,
            crate::state::ShadingMode::Normals => 2,
        }
    }

    fn shading_mode_from_byte(byte: u8) -> Option<crate::state::ShadingMode> {
        match byte {
            0 => Some(crate::state::ShadingMode::Lit),
            1 => Some(crate::state::ShadingMode::Unlit),
            2 => Some(crate::state::ShadingMode::Normals),
            _ => None,
        }
    }

    fn display_mode_byte(mode: crate::state::DisplayMode) -> u8 {
        match mode {
            crate::state::DisplayMode::Off => 0,
            crate::state::DisplayMode::On => 1,
            crate::state::DisplayMode::Only => 2,
        }
    }

    fn display_mode_from_byte(byte: u8) -> Option<crate::state::DisplayMode> {
        match byte {
            0 => Some(crate::state::DisplayMode::Off),
            1 => Some(crate::state::DisplayMode::On),
            2 => Some(crate::state::DisplayMode::Only),
            _ => None,
        }
    }
}

/// Synchronizes the browser URL query parameters with archive, file selection state, and the view state
pub fn update_query(query_state: &QueryState) {
    bevy::log::warn!("Updating query string.");
    let Some(window) = web_sys::window() else {
        bevy::log::warn!("Failed to get window object.");
        return;
    };

    let Ok(current_url) = window.location().href() else {
        bevy::log::warn!("Failed to get current URL from window location.");
        return;
    };

    let Ok(url) = web_sys::Url::new(&current_url) else {
        bevy::log::warn!("Failed to create URL from current location.");
        return;
    };

    let Ok(query) = serde_urlencoded::to_string(query_state)
        .inspect_err(|error| bevy::log::warn!("Could not serialize query state: {error}"))
    else {
        return;
    };

    url.set_search(&format!("?{query}"));
    if let Ok(history) = window.history()
        && let Err(error) = history.replace_state_with_url(&JsValue::NULL, "", Some(&url.href()))
    {
        bevy::log::warn!("Could not update page URL: {:?}", error);
    }
}

/// Reads the query state from the browser URL query string.
pub fn query_state() -> Option<QueryState> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    serde_urlencoded::from_str(search.strip_prefix('?').unwrap_or(&search)).ok()
}
