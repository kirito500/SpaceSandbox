
pub use encase::{ShaderType, private::WriteInto, UniformBuffer};
pub use space_core::SpaceResult;
pub use space_macros::unifrom_struct;

pub trait ShaderUniform : ShaderType + WriteInto {
    fn get_name(&self) -> String;
    fn get_struct(&self) -> String;
    fn get_bytes(&self) -> SpaceResult<Vec<u8>> {
        let mut camera_cpu_buffer = UniformBuffer::new(vec![]);
        camera_cpu_buffer.write(&self)?;
        Ok(camera_cpu_buffer.into_inner())
    }
}

unifrom_struct!(
    PointLightUniform,
    pos : vec3,
    color : vec3,
    intensity : f32
);

unifrom_struct!(
    LightCamera,
    pos : vec3,
    frw : vec3,
    up : vec3,
    far : f32
);