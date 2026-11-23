use std::{
    borrow::Cow,
    collections::HashMap,
    env, fs, io,
    path::{Path, PathBuf},
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
        self.0.insert(scheme.into(), Arc::new(reader));
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
        let path = self.root.join(Path::new(path));
        fs::read(path).map(Bytes::from)
    }
}

fn resolve_root(root: &Path) -> PathBuf {
    if root.is_absolute() {
        return root.to_path_buf();
    }

    if let Some(dir) = option_env!("CARGO_MANIFEST_DIR") {
        return Path::new(dir).join(root);
    }

    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(root)))
        .unwrap_or_else(|| root.to_path_buf())
}
