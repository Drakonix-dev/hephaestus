use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    env, fs, io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use bytes::Bytes;

use crate::assets::errors::InitError;

pub trait AssetReader: Send + Sync + 'static {
    fn read(&self, path: &str) -> io::Result<Bytes>;
}

pub(super) struct Readers(HashMap<Cow<'static, str>, Arc<dyn AssetReader>>);

impl Readers {
    pub(super) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(super) fn get(&self, scheme: &str) -> Option<Arc<dyn AssetReader>> {
        self.0.get(scheme).cloned()
    }

    pub(super) fn register(
        &mut self,
        scheme: impl Into<Cow<'static, str>>,
        reader: impl AssetReader,
    ) {
        let entry = self.0.entry(scheme.into());
        match entry {
            Entry::Occupied(_) => {
                panic!("duplicate reader for scheme {}", scheme)
            }

            Entry::Vacant(e) => {
                let _ = e.insert(Arc::new(reader));
            }
        }
    }
}

pub struct FileReader {
    root: PathBuf,
}

impl FileReader {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, InitError> {
        let root = resolve_root(root.as_ref());
        let root = root
            .canonicalize()
            .map_err(|source| InitError::Root { path: root, source })?;

        Ok(Self { root })
    }
}

impl AssetReader for FileReader {
    fn read(&self, path: &str) -> io::Result<Bytes> {
        // TODO: actually add security maybe instead of just checking path string
        let rel = Path::new(path.as_ref());
        if !rel
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(io::Error::other(format!(
                "`{}` escapes the asset root",
                rel.to_path_buf()
            )));
        }

        let path = self.root.join(Path::new(path));
        fs::read(path).map(Bytes::from)
    }
}

fn resolve_root(root: &Path) -> PathBuf {
    if root.is_absolute() {
        return root.to_path_buf();
    }

    if let Some(dir) = env::var_os("CARGO_MANIFEST_DIR") {
        return PathBuf::from(dir).join(root);
    }

    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(root)))
        .unwrap_or_else(|| root.to_path_buf())
}
