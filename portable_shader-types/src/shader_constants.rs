use bytemuck::{NoUninit, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, NoUninit, Zeroable)]
pub struct ShaderConstants {
  pub width: u32,
  pub height: u32,
  pub time: f32,

  // Mouse state.
  pub cursor_x: f32,
  pub cursor_y: f32,
  pub drag_start_x: f32,
  pub drag_start_y: f32,
  pub drag_end_x: f32,
  pub drag_end_y: f32,
  pub mouse_left_pressed: u32,
  pub mouse_left_clicked: u32,
}
