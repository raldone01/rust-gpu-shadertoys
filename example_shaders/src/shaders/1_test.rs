pub mod phantom_star_2 {
  use super::*;
  define_shader!({
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
