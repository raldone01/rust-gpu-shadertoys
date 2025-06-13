use shared_with_gpu::shader_definition::ShaderDefinition;

macro_rules! register_shaders {
    ($($shader_name:ident),* $(,)?) => {

        // need pub for rust-gpu otherwise the entrypoints get optimized out
        $(
            pub mod $shader_name;
        )*

        #[cfg(feature = "cpu_definition_export")]
        pub const SHADER_DEFINITIONS: &[&ShaderDefinition<'_>] = &[
            $(
                &$shader_name::SHADER_DEFINITION,
            )*
        ];
    };
}

register_shaders!(
  a_lot_of_spheres,
  miracle_snowflakes,
  /*morphing_teapot,
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
