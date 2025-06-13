use core::fmt::Debug;

use bytemuck::{NoUninit, Zeroable};
use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};

/// TODO: move this
pub(crate) fn convert_fs_input_to_shader_input(
  in_frag_coord: Vec4,
  constants: &portable_shader_types::shader_constants::ShaderConstants,
) -> LegacyShadertoyGlobals {
  let mut frag_coord = vec2(in_frag_coord.x, in_frag_coord.y);
  let resolution = vec3(constants.width as f32, constants.height as f32, 0.0);
  let time = constants.time;
  let mut mouse = vec4(
    constants.drag_end_x,
    constants.drag_end_y,
    constants.drag_start_x,
    constants.drag_start_y,
  );
  if mouse != Vec4::ZERO {
    mouse.y = resolution.y - mouse.y;
    mouse.w = resolution.y - mouse.w;
  }
  if constants.mouse_left_pressed != 1 {
    mouse.z *= -1.0;
  }
  if constants.mouse_left_clicked != 1 {
    mouse.w *= -1.0;
  }

  frag_coord.x %= resolution.x;
  frag_coord.y = resolution.y - frag_coord.y % resolution.y;

  LegacyShadertoyGlobals {
    resolution,
    time,
    frag_coord,
    mouse,
  }
}

#[doc(hidden)]
pub(crate) fn _assert_zeroable<T: Zeroable>() {}

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! define_shader {
  ({
    name: $name:literal,
    $(uniform_buffers: { $(
        $buffer_name:ident: {
            type: $buffer_type:ident,
            binding: $spirv_binding:literal
        }),* $(,)?
    },)?
    $(const_parameters: { $(
        $shader_param_name:ident: $shader_param_type:ident $({
            $($shader_param_config_field_name:ident: $shader_param_config_field_type:expr),* $(,)?
        })?
    ),* $(,)? },)?
    // TODO: support extern textures (file/url)
    // TODO: support keyboard, better mouse input, ...
  }) => {
    // must be public otherwise it is optimized out before rust-gpu can prevent that
    #[cfg(feature = "shader_code")]
    #[spirv_std::spirv(fragment(entry_point_name = $name))]
    pub fn main_fs(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(push_constant)] in_constants: &portable_shader_types::shader_constants::ShaderConstants,
    output: &mut Vec4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] in_const_parameters: &ShaderParameterValues,
    $($(#[spirv(storage_buffer, descriptor_set = 2, binding = $spirv_binding)] $buffer_name: &mut $buffer_type),* ,)?
    ) {
    let shader_input = $crate::shader_infra::convert_fs_input_to_shader_input(in_frag_coord, in_constants);
    let shader_context = ShaderContext {
        $($( $buffer_name ),* ,)?
        const_parameter_values: in_const_parameters,
        _phantom: core::marker::PhantomData,
    };
    main_image(
        &shader_input,
        &shader_context,
        output,
        shader_input.frag_coord,
    );
    }

    pub(crate) const SHADER_DEFINITION: portable_shader_types::shader_definition::ShaderDefinition<'static> = portable_shader_types::shader_definition::ShaderDefinition {
      name: $name,
      parameters: (portable_shader_types::shader_definition::MagicCowVec::Borrowed(&[
        $(
          &portable_shader_types::shader_definition::ShaderParameters::$shader_param_type<'static>($shader_param_type(portable_shader_types::shader_definition::ShaderParameters::$shader_param_type {
            $($($shader_param_config_field_name: $shader_param_config_field_type),*)?
          })),
        )*
      ])),
    };

    struct ShaderParameters {
      $($(pub $shader_param_name: $shader_param_type),* ,)?
    }

    const SHADER_PARAMETERS: ShaderParameters = ShaderParameters {
      $($($shader_param_name: $shader_param_type {
          $($($shader_param_config_field_name: $shader_param_config_field_type),*)?
      }),* ,)?
    };

    const _: () = {
      // assert Zeroable for all buffers
      fn _assert_zeroable_buffers() {
        $(_assert_zeroable::<$buffer_type>();)*
      }
    };

    #[repr(C)]
    #[derive(bytemuck::NoUninit, Copy, Clone)]
    pub struct ShaderParameterValues {
      $($(pub $shader_param_name: <$shader_param_type as ShaderParameter>::ParameterValue),* ,)?
    }

    pub struct ShaderContext<'a> {
        $($(pub $buffer_name: &'a mut $buffer_type),* ,)?
        pub const_parameter_values: &'a ShaderParameterValues,
        _phantom: core::marker::PhantomData<&'a ()>,
    }
  };
}
pub(crate) use define_shader;

pub struct LegacyShadertoyGlobals {
  pub resolution: Vec3,
  pub time: f32,
  pub frag_coord: Vec2,
  /// https://www.shadertoy.com/view/Mss3zH
  pub mouse: Vec4,
}
