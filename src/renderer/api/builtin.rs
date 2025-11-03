use std::{mem, num::NonZeroU64, path::PathBuf};

use crate::renderer::{BindGroupLayout, BindingDesc, BindingType, MaterialUniforms, ShaderDefinition, ShaderStage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinShader {
    SimpleColor,
}

pub(crate) fn builtin_shader_definitions() -> Vec<(BuiltinShader, ShaderDefinition)> {
    let simple_color = ShaderDefinition {
        source: PathBuf::from("hephaestus/assets/shaders/simple_color.wgsl"),
        layout: BindGroupLayout {
            entries: vec![BindingDesc {
                binding: 0,
                ty: BindingType::UniformBuffer {
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(
                        mem::size_of::<MaterialUniforms>() as u64,     
                    ).unwrap()),
                },
                visibility: ShaderStage::VertexFragment,
            }],
        },
    }; 

    vec![
        (BuiltinShader::SimpleColor, simple_color),
    ]
}
