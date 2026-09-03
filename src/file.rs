use bevy::ecs::system::ResMut;
use std::{collections::HashMap, io::{self, Cursor, Read}};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Response, Window};
use zip::ZipArchive;

use crate::file;

#[wasm_bindgen]
pub async fn fetch_file_from_server(url: &str) -> Result<Vec<u8>, JsValue> {
    let window = web_sys::window().ok_or("No global window found")?;

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
) -> Result<HashMap<String, Vec<u8>>, wasm_bindgen::JsValue> {
    let mut file_system: HashMap<String, Vec<u8>> = HashMap::new();
    
    let zip_bytes = fetch_file_from_server(url).await?;

    let cursor = Cursor::new(zip_bytes);
    let mut archive = ZipArchive::new(cursor).unwrap();

    // Iterate through each file inside the ZIP
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        if file.is_dir() {
            println!("Directory: {}", name);
        } else {
            println!("File: {}, size: {} bytes", name, file.size());

            // Read the file contents into a buffer or handle it as needed
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();

            file_system.insert(name, contents);
        }
    }

    // ... items from previous step: window.fetch_with_str, resp.array_buffer()
    // ... zip::ZipArchive::new(reader) loops ...
    Ok(file_system)
}
