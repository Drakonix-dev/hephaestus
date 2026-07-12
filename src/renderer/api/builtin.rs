use std::{fmt, mem, num::NonZeroU64, path::PathBuf};

use crate::renderer::{
    BindGroupLayout, BindingDesc, BindingType, MaterialUniforms, ShaderDefinition, ShaderStage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinShader {
    SimpleColor,
    UnlitTextured,
}

impl fmt::Display for BuiltinShader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BuiltinShader::SimpleColor => write!(f, "simple color shader"),
            BuiltinShader::UnlitTextured => write!(f, "unlit textured shader"),
        }
    }
}

pub(crate) fn builtin_shader_definitions() -> Vec<(BuiltinShader, ShaderDefinition)> {
    let simple_color = ShaderDefinition {
        source: PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/shaders/simple_color.wgsl"
        )),
        layout: BindGroupLayout {
            entries: vec![BindingDesc {
                binding: 0,
                ty: BindingType::UniformBuffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        NonZeroU64::new(mem::size_of::<MaterialUniforms>() as u64).unwrap(),
                    ),
                },
                visibility: ShaderStage::VertexFragment,
            }],
        },
    };

    let unlit_textured = ShaderDefinition {
        source: PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/shaders/unlit_textured.wgsl"
        )),
        layout: BindGroupLayout {
            entries: vec![
                BindingDesc {
                    binding: 0,
                    ty: BindingType::UniformBuffer {
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZeroU64::new(mem::size_of::<MaterialUniforms>() as u64).unwrap(),
                        ),
                    },
                    visibility: ShaderStage::VertexFragment,
                },
                BindingDesc {
                    binding: 1,
                    ty: BindingType::Texture2D,
                    visibility: ShaderStage::Fragment,
                },
                BindingDesc {
                    binding: 2,
                    ty: BindingType::Sampler,
                    visibility: ShaderStage::Fragment,
                },
            ],
        },
    };

    vec![
        (BuiltinShader::SimpleColor, simple_color),
        (BuiltinShader::UnlitTextured, unlit_textured),
    ]
}
