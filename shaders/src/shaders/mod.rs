use bytemuck::{NoUninit, Zeroable};

use crate::shader_prelude::*;

mod a_lot_of_spheres;
mod a_question_of_time;
mod apollonian;
mod atmosphere_system_test;
mod bubble_buckey_balls;
mod clouds;
mod filtering_procedurals;
mod flappy_bird;
mod galaxy_of_universes;
mod geodesic_tiling;
mod heart;
mod luminescence;
mod mandelbrot_smooth;
mod miracle_snowflakes;
mod morphing_teapot;
mod moving_square;
mod on_off_spikes;
mod phantom_star;
mod playing_marble;
mod protean_clouds;
mod raymarching_primitives;
mod seascape;
mod skyline;
mod soft_shadow_variation;
mod tileable_water_caustic;
mod tokyo;
mod two_tweets;
mod voxel_pac_man;

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! match_index {
    ($e:expr; $($result:expr),* $(,)?) => ({
        let mut i = 0..;
        match $e { e => {
            $(if e == i.next().unwrap() { $result } else)*
            { unreachable!() }
        }}
    })
}

macro_rules! render_shader_macro {
    ($($shader_name:path),* $(,)?) => {
        #[inline(always)]
        pub fn render_shader(shader_index: u32, shader_input: &ShaderInput, shader_output: &mut ShaderResult) {
            match_index!(shader_index; $(
                <$shader_name>::shader_fn(shader_input, shader_output),
            )*)
        }

        pub const SHADER_DEFINITIONS: &[&ShaderDefinition] = &[
            $(
                <$shader_name>::SHADER_DEFINITION,
            )*
        ];
    };
}

render_shader_macro!(
  a_lot_of_spheres::ShaderALotOfSpheres,
  /*miracle_snowflakes::ShaderMiracleSnowflakes<'_>,
  morphing_teapot::ShaderMorphingTeapot<'_>,
  voxel_pac_man,
  luminescence,
  seascape,
  two_tweets,
  heart,
  clouds,
  mandelbrot_smooth,
  protean_clouds,
  tileable_water_caustic,
  apollonian,
  phantom_star,
  playing_marble,
  a_lot_of_spheres,
  a_question_of_time,
  galaxy_of_universes,
  atmosphere_system_test,
  soft_shadow_variation,
  bubble_buckey_balls,
  raymarching_primitives,
  moving_square,
  skyline,
  filtering_procedurals,
  geodesic_tiling,
  flappy_bird,
  tokyo,
  on_off_spikes,*/
);

fn convert_fs_input_to_shader_input(
  in_frag_coord: Vec4,
  constants: &ShaderConstants,
) -> ShaderInput {
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

  ShaderInput {
    resolution,
    time,
    frag_coord,
    mouse,
  }
}

pub trait ShaderParameter {
  type ParameterValue: NoUninit;
}

pub struct ParameterIntSlider {
  min: i32,
  max: i32,
  default: i32,
  label: &'static str,
  description: &'static str,
  step: i32,
}

impl ShaderParameter for ParameterIntSlider {
  type ParameterValue = i32;
}

#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! define_shader {
  (pub struct ShaderDefinition {
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
    #[spirv(fragment(entry_point_name = $name))]
    pub fn main_fs(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(push_constant)] in_constants: &ShaderConstants,
    output: &mut Vec4,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] in_const_parameters: &ShaderParameterValues,
    $($(#[spirv(storage_buffer, descriptor_set = 2, binding = $spirv_binding)] $buffer_name: &mut $buffer_type),* ,)?
    ) {
    let shader_input = convert_fs_input_to_shader_input(in_frag_coord, in_constants);
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

    struct ShaderParameters {
      $($(pub $shader_param_name: $shader_param_type),* ,)?
    }

    const SHADER_PARAMETERS: ShaderParameters = ShaderParameters {
      $($($shader_param_name: $shader_param_type {
          $($($shader_param_config_field_name: $shader_param_config_field_type),*)?
      }),* ,)?
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

pub mod phantom_star_2 {
  use super::*;
  define_shader!(pub struct ShaderDefinition {
    name: "Phantom Star",
    uniform_buffers: {
        buffer_a: { type: BufferA, binding: 0 },
        buffer_b: { type: BufferB, binding: 1 },
    },
    const_parameters: {
        my_int_slider: ParameterIntSlider {
            min: 0,
            max: 100,
            default: 50,
            label: "My Int Slider",
            description: "This is a slider for an integer value.",
            step: 1,
        },
    },
  });

  #[derive(Zeroable)]
  pub struct BufferA {
    my_data: [f32; 3],
  }

  #[derive(Zeroable)]
  pub struct BufferB {
    my_other_data: [f32; 3],
  }

  fn main_image(
    shader_input: &ShaderInput,
    shader_context: &ShaderContext<'_>,
    frag_color: &mut Vec4,
    frag_coord: Vec2,
  ) {
  }
}
