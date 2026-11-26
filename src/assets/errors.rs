use std::{error::Error, io, path::PathBuf, sync::Arc};

use crate::assets::{AssetLabel, AssetPath};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("failed to build {asset}")]
    Build {
        asset: AssetLabel,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("{asset} depends on {dependency}, which failed")]
    DependencyFailed {
        asset: AssetLabel,
        dependency: AssetLabel,
        #[source]
        source: Arc<AssetError>,
    },

    #[error("{asset}: loader panicked: {message}")]
    Panicked { asset: AssetLabel, message: String },

    #[error("failed to parse {asset}")]
    Parse {
        asset: AssetLabel,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("failed to read `{path}`")]
    Read {
        path: AssetPath,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GraphError {
    #[error("cycle detected")]
    Cycle,

    #[error("self referential cycle found")]
    SelfReference,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InitError {
    #[error("asset root `{}` is not accessible", path.display())]
    Root {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
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
