use std::path::PathBuf;

pub struct AssetConfig {
    /// Root for the default `file://` reader. Relative paths resolve against
    /// CARGO_MANIFEST_DIR when run by cargo.
    pub root: PathBuf,
    /// Threads that run blocking reads.
    pub io_threads: u8,
    /// Threads that run CPU-side parsing, `None` = available_parallelism - 1 (min 1).
    pub parse_threads: Option<usize>,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("assets"),
            io_threads: 4,
            parse_threads: None,
        }
    }
}
