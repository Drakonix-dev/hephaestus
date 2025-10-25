use std::path::Path;

use crate::define_handle;

// ShaderHandle defines a handle for a specific shader.
define_handle!(ShaderHandle);

pub struct ShaderDefinition<'a> {
    pub source: &'a Path,
}
