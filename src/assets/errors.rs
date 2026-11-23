use std::{error::Error, io, path::PathBuf, sync::Arc};

use crate::assets::AssetLabel;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("failed to build {asset}")]
    Build {
        asset: AssetLabel,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("handle released before load finished")]
    Cancelled,

    #[error("{asset} depends on {dependency}, which failed")]
    DependencyFailed {
        asset: AssetLabel,
        dependency: &'static str,
        #[source]
        source: Arc<AssetError>,
    },

    #[error("failed to read `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse {asset}")]
    Parse {
        asset: AssetLabel,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("`{path}` escapes the asset root")]
    PathOutsideRoot { path: PathBuf },

    #[error("superseded by a newer load")]
    Superseded,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    #[error("no loader registered for {asset} from {desc}")]
    UnhandledAsset {
        asset: &'static str,
        desc: &'static str,
    },

    #[error("unknown path scheme `{scheme}` in `{path}`")]
    UnknownScheme { path: String, scheme: String },
}
