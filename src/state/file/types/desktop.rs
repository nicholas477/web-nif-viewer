use super::Filesystem;
use arc_slice::ArcSlice;
use std::fs::File;
use std::sync::RwLock;
use std::{collections::HashMap, future::Future, pin::Pin};

/// A filesystem implementation that reads files from the real filesystem and caches them in memory.
#[derive(Default)]
pub struct RealFS {
    /// Base path to add to all file lookups
    base: RwLock<String>,

    /// Inner hash map storing the file data in memory
    inner: RwLock<HashMap<String, ArcSlice<[u8]>>>,
}

impl RealFS {
    pub fn new(base: String) -> Self {
        Self {
            base: RwLock::new(base),
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Filesystem for RealFS {
    fn read<'a>(
        &'a self,
        path: &'a str,
        _absolute_path: bool,
    ) -> Pin<Box<dyn Future<Output = Option<ArcSlice<[u8]>>> + 'a>> {
        Box::pin(async move {
            let full_path = format!("{}/{}", self.base.read().unwrap(), path);

            if let Some(data) = self.inner.read().unwrap().get(&full_path).cloned() {
                return Some(data);
            }

            if let Ok(mut file) = File::open(&full_path) {
                use std::io::Read;
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    let data = ArcSlice::from(buffer.as_slice());
                    self.inner
                        .write()
                        .unwrap()
                        .insert(path.to_string(), data.clone());
                    return Some(data);
                }
            }
            None
        })
    }

    fn absolute_paths(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }

    fn has_base(&self) -> bool {
        true
    }

    fn set_base(&self, base: String) {
        *self.base.write().unwrap() = base;
    }
}
