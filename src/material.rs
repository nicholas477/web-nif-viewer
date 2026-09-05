use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

const PHONG_SHADER_PATH: &str = "shaders/phong.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(PhongMaterialKey)]
pub struct PhongMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub color_texture: Option<Handle<Image>>,
    #[uniform(3)]
    pub settings: Vec4,
    pub alpha_mode: AlphaMode,
    pub cull_mode: Option<Face>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PhongMaterialKey {
    cull_mode: Option<Face>,
}

impl From<&PhongMaterial> for PhongMaterialKey {
    fn from(material: &PhongMaterial) -> Self {
        Self {
            cull_mode: material.cull_mode,
        }
    }
}

impl Material for PhongMaterial {
    fn fragment_shader() -> ShaderRef {
        PHONG_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        Ok(())
    }
}
