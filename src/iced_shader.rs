use std::{sync::Arc, time::Instant};

use iced_core::{layout, mouse, renderer, Rectangle, Size, Widget};
use iced_wgpu::{graphics::Viewport, primitive::Renderer};
use iced_widget::shader;
use shadertoys_shaders::shared_data::{self, ShaderConstants};

#[derive(Clone)]
pub struct ShaderToyShader {
  shader_module: Arc<wgpu::ShaderModule>,
}

impl ShaderToyShader {
  #[must_use]
  pub fn new(shader_module: Arc<wgpu::ShaderModule>) -> Self {
    Self { shader_module }
  }
}

impl<Message> shader::Program<Message> for ShaderToyShader {
  type State = ();
  type Primitive = Primitive;

  fn draw(
    &self,
    _state: &Self::State,
    _cursor: mouse::Cursor,
    _bounds: Rectangle,
  ) -> Self::Primitive {
    Primitive::new(self.shader_module.clone())
  }
}

#[derive(Debug)]
pub struct Primitive {
  shader_module: Arc<wgpu::ShaderModule>,
}

impl Primitive {
  pub fn new(shader_module: Arc<wgpu::ShaderModule>) -> Self {
    Self { shader_module }
  }
}

impl shader::Primitive for Primitive {
  fn prepare(
    &self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    storage: &mut shader::Storage,
    _bounds: &Rectangle,
    viewport: &Viewport,
  ) {
    if !storage.has::<Pipeline>() {
      storage.store(Pipeline::new(
        device,
        queue,
        format,
        "myshader_name",
        self.shader_module.clone(),
      ));
    }

    let pipeline = storage.get_mut::<Pipeline>().unwrap();

    // Upload data to GPU
    pipeline.update(device, queue, viewport.physical_size());
  }

  fn render(
    &self,
    encoder: &mut wgpu::CommandEncoder,
    storage: &shader::Storage,
    target: &wgpu::TextureView,
    clip_bounds: &Rectangle<u32>,
  ) {
    // At this point our pipeline should always be initialized
    let pipeline = storage.get::<Pipeline>().unwrap();

    // Render primitive
    pipeline.render(target, encoder, *clip_bounds);
  }
}

pub struct Pipeline {
  render_pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
  pub fn new(
    device: &wgpu::Device,
    _queue: &wgpu::Queue,
    swapchain_format: wgpu::TextureFormat,
    shader_name: &str,
    shader_module: Arc<wgpu::ShaderModule>,
  ) -> Self {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some(shader_name),
      bind_group_layouts: &[],
      push_constant_ranges: &[wgpu::PushConstantRange {
        stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
        range: 0..std::mem::size_of::<shared_data::ShaderConstants>() as u32,
      }],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some(shader_name),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader_module,
        entry_point: "main_vs",
        buffers: &[],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader_module,
        entry_point: "main_fs",
        targets: &[Some(wgpu::ColorTargetState {
          format: swapchain_format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
    });

    Self { render_pipeline }
  }

  /// TODO: supply buffers here
  pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target_size: Size<u32>) {}

  pub fn render(
    &self,
    target: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
    viewport: Rectangle<u32>,
  ) {
    {
      let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("shadertoy.pipeline.pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: target,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      rpass.set_viewport(
        viewport.x as f32,
        viewport.y as f32,
        viewport.width as f32,
        viewport.height as f32,
        0.0,
        1.0,
      );
      // TODO: precompute shader constants
      let push_constants = ShaderConstants {
        width: viewport.width,
        height: viewport.height,
        time: Instant::now().elapsed().as_secs_f32(),
        cursor_x: 0.,
        cursor_y: 0.,
        drag_start_x: 0.,
        drag_start_y: 0.,
        drag_end_x: 0.,
        drag_end_y: 0.,
        mouse_left_pressed: 0 as u32,
        mouse_left_clicked: 0 as u32,
      };
      rpass.set_pipeline(&self.render_pipeline);
      rpass.set_push_constants(
        wgpu::ShaderStages::VERTEX_FRAGMENT,
        0,
        bytemuck::bytes_of(&push_constants),
      );
      rpass.draw(0..3, 0..1);
    }
  }
}
