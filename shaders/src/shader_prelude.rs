pub use core::f32::consts::{FRAC_1_PI, FRAC_PI_2, PI, TAU};

/// We can't use the `f32::consts::SQRT_3` constant here because it is an unstable library feature
pub const SQRT_3: f32 = 1.732_050_807_568_877_2;

pub use spirv_std::glam::{
  mat2, mat3, vec2, vec3, vec4, Mat2, Mat3, Vec2, Vec2Swizzles, Vec3, Vec3Swizzles, Vec4,
  Vec4Swizzles,
};

pub use crate::{
  shader_infra::LegacyShadertoyGlobals, shader_std::*, shared_data::ShaderConstants,
};

pub(crate) use crate::shader_infra::define_shader;
