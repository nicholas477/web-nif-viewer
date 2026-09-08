use std::pin::Pin;

/// Async read result for a file opened by the file picker dialog
pub type ReadResult = Pin<Box<dyn Future<Output = Option<Vec<u8>>>>>;
pub trait PickerFile: std::fmt::Debug {
    fn name(&self) -> String;
    fn read(&self) -> ReadResult;
    fn mime_type(&self) -> String;
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::path::PathBuf;

    use rfd::FileDialog;

    #[derive(Debug)]
    struct File {
        path: PathBuf,
    }

    impl super::PickerFile for File {
        fn name(&self) -> String {
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string()
        }

        fn read(&self) -> super::ReadResult {
            let path = self.path.clone();
            Box::pin(async move { std::fs::read(path).ok() })
        }

        fn mime_type(&self) -> String {
            match self.path.extension().and_then(|extension| extension.to_str()) {
                Some("zip") => "application/zip".to_string(),
                Some("nif") => "application/octet-stream".to_string(),
                _ => String::new(),
            }
        }
    }

    /// Opens the native file picker and returns the selected file.
    pub async fn pick_single_file(accepts: &str) -> Option<Box<dyn super::PickerFile>> {
        let extensions: Vec<_> = accepts
            .split(',')
            .filter_map(|accept| accept.trim().strip_prefix('.'))
            .collect();

        let dialog = FileDialog::new();
        let dialog = if extensions.is_empty() {
            dialog
        } else {
            dialog.add_filter("Supported files", &extensions)
        };

        dialog
            .pick_file()
            .map(|path| Box::new(File { path }) as Box<dyn super::PickerFile>)
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use bevy::prelude::*;
    use futures::channel::oneshot;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::{JsCast, closure::Closure};
    use web_sys::File;
    use web_sys::{HtmlInputElement, window};

    impl super::PickerFile for File {
        fn name(&self) -> String {
            self.name()
        }

        fn read(&self) -> super::ReadResult {
            let promise = self.array_buffer();
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            Box::pin(async move {
                match future.await {
                    Ok(js_value) => {
                        let array = js_sys::Uint8Array::new(&js_value);
                        let mut vec = vec![0; array.length() as usize];
                        array.copy_to(&mut vec);
                        Some(vec)
                    }
                    Err(_) => None,
                }
            })
        }

        fn mime_type(&self) -> String {
            self.type_()
        }
    }

    /// Opens the browser's file picker for uploading a file. Returns the file
    pub async fn pick_single_file(accepts: &str) -> Option<Box<dyn super::PickerFile>> {
        let window = window()?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("No document exists"))
            .ok()?;

        let input_element = document
            .create_element("input")
            .ok()?
            .dyn_into::<HtmlInputElement>()
            .ok()?;

        input_element.set_type("file");
        input_element.set_multiple(false);
        input_element.set_accept(accepts);

        // Create a channel to send the file back when the user selects it
        let (sender, receiver) = oneshot::channel::<Option<File>>();
        let mut sender = Some(sender);

        let input_clone = input_element.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if let Some(tx) = sender.take() {
                let files = input_clone.files();
                let file = files.and_then(|f| f.get(0));
                let _ = tx.send(file); // Send file back (or None if canceled/empty)
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        input_element
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .ok()?;
        closure.forget(); // Keep closure alive until the event fires

        // Open the browser dialog
        input_element.click();

        // Wait asynchronously for the user to select the file
        let file = receiver.await.ok()?;

        file.map(|f| Box::new(f) as Box<dyn super::PickerFile>)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;

#[cfg(target_arch = "wasm32")]
pub use web::*;
