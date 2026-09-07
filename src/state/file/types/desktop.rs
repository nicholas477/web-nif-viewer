use std::sync::RwLock;
use std::collections::HashMap;
use std::fs::File;
use arc_slice::ArcSlice;
use super::Filesystem;

/// A filesystem implementation that reads files from the real filesystem and caches them in memory.
#[derive(Default)]
pub struct RealFS {
    /// Base path to add to all file lookups
    base: String,

    /// Inner hash map storing the file data in memory
    inner: RwLock<HashMap<String, ArcSlice<[u8]>>>
}

impl RealFS {
    pub fn new(base: String) -> Self {
        Self {
            base,
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Filesystem for RealFS {
    fn read(&self, path: &str) -> Option<ArcSlice<[u8]>> {
        let full_path = format!("{}/{}", self.base, path);
        if let Some(data) = self.inner.read().unwrap().get(&full_path).cloned() {
            return Some(data);
        }
        
        if let Ok(mut file) = File::open(&full_path) {
            use std::io::Read;
            let mut buffer = Vec::new();
            if file.read_to_end(&mut buffer).is_ok() {
                let data = ArcSlice::from(buffer.as_slice());
                self.inner.write().unwrap().insert(full_path, data.clone());
                return Some(data);
            }
        }
        None
    }

    fn paths(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }
}