use arc_slice::ArcSlice;
use std::future::Future;
use std::sync::RwLock;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::{collections::HashMap, pin::pin};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions,
};

use super::Filesystem;

/// A filesystem implementation that reads files from the real filesystem and caches them in memory.
/// On wasm, it interacts with the browser's File System Access API to read files.
pub struct RealFS {
    /// The handle on the filesystem directory
    fs_handle: FileSystemDirectoryHandle,

    /// Inner hash map storing the file data in memory
    inner: RwLock<HashMap<String, ArcSlice<[u8]>>>,
}

impl RealFS {
    pub fn new(fs_handle: FileSystemDirectoryHandle) -> Self {
        Self {
            fs_handle,
            inner: RwLock::new(HashMap::new()),
        }
    }
}

// A simple thread-blocking waker
struct ThreadWaker {
    lock: Mutex<bool>,
    cvar: Condvar,
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        let mut started = self.lock.lock().unwrap();
        *started = true;
        self.cvar.notify_one(); // Wake up the looping thread
    }
}

fn block_on<F: Future>(fut: F) -> F::Output {
    // 1. Pin the future to the stack
    let mut pinned_fut = pin!(fut);

    // 2. Create our custom thread-blocking waker
    let my_waker = Arc::new(ThreadWaker {
        lock: Mutex::new(false),
        cvar: Condvar::new(),
    });
    let waker = Waker::from(my_waker.clone());
    let mut cx = Context::from_waker(&waker);

    // 3. The Polling Loop
    loop {
        match pinned_fut.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                // Wait efficiently until the waker notifies us
                let mut started = my_waker.lock.lock().unwrap();
                while !*started {
                    started = my_waker.cvar.wait(started).unwrap();
                }
                *started = false; // Reset for the next pending state
            }
        }
    }
}

/// Resolves a deep file path starting from a root FileSystemDirectoryHandle.
/// Example path: "Data Files/Morrowind.esm"
async fn get_file_by_path(
    root_dir: &FileSystemDirectoryHandle,
    file_path: &str,
) -> Result<FileSystemFileHandle, JsValue> {
    // 1. Split path into parts, filtering out any empty strings
    let parts: Vec<&str> = file_path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        return Err(JsValue::from_str("Provided path is empty"));
    }

    // Isolate the final filename from the folder paths
    let (dir_parts, file_name) = parts.split_at(parts.len() - 1);
    let target_file_name = file_name[0];

    // 2. Traverse down the subdirectory tree sequentially
    let mut current_dir = root_dir.clone();
    let dir_options = FileSystemGetDirectoryOptions::new(); // Defaults: create = false

    for sub_dir_name in dir_parts {
        let dir_promise = current_dir.get_directory_handle_with_options(sub_dir_name, &dir_options);
        let next_dir_js = JsFuture::from(dir_promise).await?;
        current_dir = next_dir_js.dyn_into::<FileSystemDirectoryHandle>()?;
    }

    // 3. Retrieve the terminal file handle from the final directory
    let file_options = FileSystemGetFileOptions::new(); // Defaults: create = false
    let file_promise = current_dir.get_file_handle_with_options(target_file_name, &file_options);
    let file_handle_js = JsFuture::from(file_promise).await?;

    let file_handle = file_handle_js.dyn_into::<FileSystemFileHandle>()?;
    Ok(file_handle)
}

async fn get_bytes_from_file_handle(
    file_handle: &FileSystemFileHandle,
) -> Result<Vec<u8>, JsValue> {
    // 1. Get the standard web File object from the browser's file handle
    let file_promise = file_handle.get_file();
    let file_js = JsFuture::from(file_promise).await?;
    let file = file_js.dyn_into::<File>()?;

    // 2. Load the file's binary contents into an ArrayBuffer
    let array_buffer_promise = file.array_buffer();
    let array_buffer_js = JsFuture::from(array_buffer_promise).await?;
    let array_buffer = array_buffer_js.dyn_into::<js_sys::ArrayBuffer>()?;

    // 3. Create a view over the ArrayBuffer and copy it into a native Rust Vec
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let mut bytes = vec![0u8; uint8_array.length() as usize];
    uint8_array.copy_to(&mut bytes);

    Ok(bytes)
}

impl Filesystem for RealFS {
    fn read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        if let Some(data) = self.inner.read().unwrap().get(path).cloned() {
            return Some(data);
        }

        let res = block_on(get_file_by_path(&self.fs_handle, path));
        if let Some(bytes_arc) = match res {
            Ok(file_handle) => {
                let bytes = block_on(get_bytes_from_file_handle(&file_handle));
                match bytes {
                    Ok(data) => Some(ArcSlice::from(data.as_slice())),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        } {
            self.inner
                .write()
                .unwrap()
                .insert(path.to_string(), bytes_arc.clone());
            Some(bytes_arc)
        } else {
            None
        }
    }

    fn paths(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }
}
