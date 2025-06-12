use clap::{command, Arg};
use futures::executor::block_on;
use iced_core::{Element, Font, Pixels, Widget};
use iced_wgpu::graphics::{futures::Subscription, Viewport};
use iced_widget::{container, runtime::Task, Text};
use iced_winit::conversion;
use ouroboros::self_referencing;
use shadertoys_shaders::{
  shaders::SHADER_DEFINITIONS,
  shared_data::{self, ShaderConstants},
};
use std::{
  cell::Cell, error::Error, fmt::Display, io, marker::PhantomData, str::FromStr, sync::Arc,
  time::Instant,
};
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;
use wgpu::{self, include_spirv, include_spirv_raw, InstanceDescriptor};
use winit::{
  application::ApplicationHandler,
  dpi::LogicalSize,
  event::{ElementState, KeyEvent, MouseButton, WindowEvent},
  event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
  keyboard::{KeyCode, ModifiersState, NamedKey, PhysicalKey},
  platform::x11::WindowAttributesExtX11,
  window::{Window, WindowAttributes, WindowId},
};

// https://book.iced.rs/index.html
// https://github.com/iced-rs/iced/blob/latest/examples/integration/src/main.rs
// https://github.com/iced-rs/iced/blob/master/winit/src/lib.rs#L133

#[self_referencing]
struct WindowSurface {
  window: Arc<Window>,
  #[borrows(window)]
  #[covariant]
  surface: wgpu::Surface<'this>,
}

struct LegacyShaderToyApp {
  device: Option<wgpu::Device>,
  queue: Option<wgpu::Queue>,
  window_surface: Option<WindowSurface>,
  config: Option<wgpu::SurfaceConfiguration>,
  render_pipeline: Option<wgpu::RenderPipeline>,
  shader_module: Option<wgpu::ShaderModule>,
  close_requested: bool,
  start: Instant,

  // UI state
  grid_mode: bool,
  shader_to_show: u32,

  // Mouse state.
  cursor_x: f32,
  cursor_y: f32,
  drag_start_x: f32,
  drag_start_y: f32,
  drag_end_x: f32,
  drag_end_y: f32,
  mouse_left_pressed: bool,
  mouse_left_clicked: bool,
}
impl LegacyShaderToyApp {
  fn render(&mut self) {
    let window_surface = match &self.window_surface {
      Some(ws) => ws,
      None => return,
    };

    let window = window_surface.borrow_window();
    let current_size = window.inner_size();
    let surface = window_surface.borrow_surface();
    let device = self.device.as_ref().unwrap();
    let queue = self.queue.as_ref().unwrap();
    let frame = match surface.get_current_texture() {
      Ok(frame) => frame,
      Err(e) => {
        error!("Failed to acquire texture: {:?}", e);
        return;
      },
    };
    let view = frame
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder =
      device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
      });
      rpass.set_viewport(
        0.0,
        0.0,
        current_size.width as f32,
        current_size.height as f32,
        0.0,
        1.0,
      );
      let push_constants = ShaderConstants {
        width: current_size.width,
        height: current_size.height,
        time: self.start.elapsed().as_secs_f32(),
        shader_display_mode: todo!(), // self.display_mode(),
        cursor_x: self.cursor_x,
        cursor_y: self.cursor_y,
        drag_start_x: self.drag_start_x,
        drag_start_y: self.drag_start_y,
        drag_end_x: self.drag_end_x,
        drag_end_y: self.drag_end_y,
        mouse_left_pressed: self.mouse_left_pressed as u32,
        mouse_left_clicked: self.mouse_left_clicked as u32,
      };
      self.mouse_left_clicked = false;
      rpass.set_pipeline(self.render_pipeline.as_ref().unwrap());
      rpass.set_push_constants(
        wgpu::ShaderStages::VERTEX_FRAGMENT,
        0,
        bytemuck::bytes_of(&push_constants),
      );
      rpass.draw(0..3, 0..1);
    }
    queue.submit(Some(encoder.finish()));
    frame.present();
  }
}

impl ApplicationHandler for LegacyShaderToyApp {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {}
  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::CloseRequested => self.close_requested = true,
      WindowEvent::Resized(new_size) => {
        if let Some(config) = self.config.as_mut() {
          config.width = new_size.width;
          config.height = new_size.height;
          if let Some(ws) = &self.window_surface {
            let surface = ws.borrow_surface();
            if let Some(device) = self.device.as_ref() {
              surface.configure(device, config);
            }
          }
        }
      },
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_x = position.x as f32;
        self.cursor_y = position.y as f32;
        if self.mouse_left_pressed {
          self.drag_end_x = self.cursor_x;
          self.drag_end_y = self.cursor_y;
        }
      },
      WindowEvent::MouseInput { state, button, .. } => {
        if button == MouseButton::Left {
          self.mouse_left_pressed = state == ElementState::Pressed;
          if self.mouse_left_pressed {
            self.drag_start_x = self.cursor_x;
            self.drag_start_y = self.cursor_y;
            self.drag_end_x = self.cursor_x;
            self.drag_end_y = self.cursor_y;
            self.mouse_left_clicked = true;
          }
        }
      },
      WindowEvent::MouseWheel { delta, .. } => {
        if let winit::event::MouseScrollDelta::LineDelta(delta_x, delta_y) = delta {
          self.drag_end_x = delta_x * 0.1;
          self.drag_end_y = delta_y * 0.1;
        }
      },
      WindowEvent::KeyboardInput { event, .. } => match event {
        KeyEvent {
          state: ElementState::Pressed,
          ..
        } if event.logical_key == NamedKey::Escape => {
          self.close_requested = true;
        },
        KeyEvent {
          state: ElementState::Pressed,
          physical_key: PhysicalKey::Code(KeyCode::KeyE),
          ..
        } => {
          self.grid_mode = false;
          self.shader_to_show = (self.shader_to_show + 1) % SHADER_DEFINITIONS.len() as u32;
          println!(
            "Shader to show: {}",
            SHADER_DEFINITIONS[self.shader_to_show as usize].name
          );
        },
        KeyEvent {
          state: ElementState::Pressed,
          physical_key: PhysicalKey::Code(KeyCode::KeyQ),
          ..
        } => {
          self.grid_mode = false;
          self.shader_to_show = (self.shader_to_show + SHADER_DEFINITIONS.len() as u32 - 1)
            % SHADER_DEFINITIONS.len() as u32;
          println!(
            "Shader to show: {}",
            SHADER_DEFINITIONS[self.shader_to_show as usize].name
          );
        },
        KeyEvent {
          state: ElementState::Pressed,
          physical_key: PhysicalKey::Code(KeyCode::KeyG),
          ..
        } => {
          self.grid_mode = !self.grid_mode;
          println!("Grid mode: {}", self.grid_mode);
        },
        _ => {},
      },
      WindowEvent::RedrawRequested => self.render(),
      _ => {},
    }
    if self.close_requested {
      event_loop.exit();
    } else if let Some(ws) = &self.window_surface {
      ws.borrow_window().request_redraw();
    }
    event_loop.set_control_flow(ControlFlow::Poll);
  }

  fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    if self.close_requested {
      event_loop.exit();
    } else if let Some(ws) = &self.window_surface {
      ws.borrow_window().request_redraw();
    }
    event_loop.set_control_flow(ControlFlow::Poll);
  }
}

#[derive(Debug, Clone)]
enum Message {}

#[derive(Debug, Clone)]
struct DynamicError(String);

impl Error for DynamicError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }

  fn description(&self) -> &str {
    "description() is deprecated; use Display"
  }

  fn cause(&self) -> Option<&dyn Error> {
    self.source()
  }
}

impl Display for DynamicError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "DynamicError: {}", self.0)
  }
}

struct IcedStuff {
  engine: iced_wgpu::Engine,
  renderer: iced_wgpu::Renderer,
  viewport: Viewport,
  debug: iced_widget::runtime::Debug,
}

impl IcedStuff {
  #[must_use]
  fn new(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    swapchain_format: wgpu::TextureFormat,
    window: &Arc<Window>,
  ) -> Self {
    // the Engine holds all the wgpu pipelines it needs
    let debug = iced_widget::runtime::Debug::new(); // controlled via the iced_runtime/debug feature
    let engine = iced_wgpu::Engine::new(&adapter, &device, &queue, swapchain_format, None);
    let renderer = iced_wgpu::Renderer::new(&device, &engine, Font::default(), Pixels::from(16));
    let physical_size = window.inner_size();
    let viewport = Viewport::with_physical_size(
      iced_core::Size::new(physical_size.width, physical_size.height),
      window.scale_factor(),
    );

    Self {
      engine,
      renderer,
      viewport,
      debug,
    }
  }
}

struct WGPURenderingStuff {
  device: wgpu::Device,
  surface_configuration: wgpu::SurfaceConfiguration,
  window_surface: WindowSurface,
  queue: wgpu::Queue,
  shader_module: wgpu::ShaderModule,
  iced_stuff: IcedStuff,
}

impl WGPURenderingStuff {
  #[must_use]
  async fn new(window_box: Arc<Window>) -> Result<Self, Box<dyn Error>> {
    // --- WGPU Instance Flags ---
    let mut wgpu_instance_flags = wgpu::InstanceFlags::default();
    // Turn off validation as the shaders are trusted.
    wgpu_instance_flags.remove(wgpu::InstanceFlags::VALIDATION);
    // Disable debugging info to speed things up.
    wgpu_instance_flags.remove(wgpu::InstanceFlags::DEBUG);

    let backend = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all());

    let instance = wgpu::Instance::new(InstanceDescriptor {
      backends: backend,
      flags: wgpu_instance_flags,
      ..Default::default()
    });

    // --- Create Window Surface ---
    let window_surface = WindowSurfaceTryBuilder {
      window: window_box,
      surface_builder: |window| instance.create_surface(window.as_ref()),
    }
    .try_build()?;
    let window = window_surface.borrow_window();
    let surface = window_surface.borrow_surface();

    // --- Request Adapter for our window ---
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
      })
      .await
      .ok_or_else(|| DynamicError(format!("Failed to request adapter!")))?;

    // --- Enable Optional Wanted Features ---
    let mut required_features = wgpu::Features::PUSH_CONSTANTS;
    if adapter
      .features()
      .contains(wgpu::Features::SPIRV_SHADER_PASSTHROUGH)
    {
      required_features |= wgpu::Features::SPIRV_SHADER_PASSTHROUGH;
    }

    // --- Required Limits ---
    let required_limits = wgpu::Limits {
      max_push_constant_size: 256,
      ..Default::default()
    };

    // --- Request Device ---
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          label: None,
          required_features,
          required_limits,
          ..Default::default()
        },
        None,
      )
      .await?;

    // --- Create Shader Module ---
    let shader_module = if device
      .features()
      .contains(wgpu::Features::SPIRV_SHADER_PASSTHROUGH)
    {
      let x = include_spirv_raw!(env!("shadertoys_shaders.spv"));
      unsafe { device.create_shader_module_spirv(&x) }
      // Newer egpu version
      //unsafe { device.create_shader_module_passthrough(x) }
    } else {
      device.create_shader_module(include_spirv!(env!("shadertoys_shaders.spv")))
    };

    // --- Setup the surface configuration ---
    let surface_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = surface_capabilities
      .formats
      .iter()
      .copied()
      .find(wgpu::TextureFormat::is_srgb)
      .or_else(|| surface_capabilities.formats.first().copied())
      .expect("Get preferred format");

    let surface_size = window.inner_size();
    let surface_configuration = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: swapchain_format,
      width: surface_size.width,
      height: surface_size.height,
      present_mode: wgpu::PresentMode::AutoVsync,
      alpha_mode: wgpu::CompositeAlphaMode::Auto,
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };
    window_surface
      .borrow_surface()
      .configure(&device, &surface_configuration);

    let iced_stuff = IcedStuff::new(&adapter, &device, &queue, swapchain_format, window);

    Ok(Self {
      device,
      surface_configuration,
      window_surface,
      queue,
      shader_module,
      iced_stuff,
    })
  }
}

struct WinitRunner {
  // gui rendering stuff
  window: Option<Arc<Window>>,
  /// TODO move this into a struct together with window to avoid separate options
  window_clipboard: Option<iced_winit::Clipboard>,
  window_renderer_stuff: Option<WGPURenderingStuff>,
  cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
  //clipboard: Clipboard, // TODO: wrap it and implement iced_core::Clipboard trait
  modifiers: ModifiersState,
  iced_state: Option<iced_runtime::program::State<ShaderToyApp>>,
}

impl WinitRunner {
  #[must_use]
  fn new() -> Self {
    Self {
      window: None,
      window_clipboard: None,
      window_renderer_stuff: None,
      cursor_position: None,
      modifiers: ModifiersState::default(),
      iced_state: None,
    }
  }

  fn get_main_window(&mut self, event_loop: &ActiveEventLoop) -> Arc<Window> {
    if let Some(main_window) = &self.window {
      main_window.clone()
    } else {
      let main_window_attributes = WindowAttributes::default()
        .with_title("Shadertoy - Rust GPU")
        .with_base_size(LogicalSize::new(1280.0, 720.0))
        // we only set it visible once we can render to it to avoid showing garbage data
        .with_visible(false);
      let main_window = Arc::from(
        event_loop
          .create_window(main_window_attributes)
          .expect("Failed to create main window"),
      );
      self.window = Some(main_window.clone());
      self.window_clipboard = Some(iced_winit::Clipboard::connect(main_window.clone()));
      main_window
    }
  }

  fn resize_main_window(&mut self, new_inner_size: winit::dpi::PhysicalSize<u32>) {
    if let Some(renderer_stuff) = &mut self.window_renderer_stuff {
      renderer_stuff.surface_configuration.width = new_inner_size.width;
      renderer_stuff.surface_configuration.height = new_inner_size.height;
      let surface = renderer_stuff.window_surface.borrow_surface();
      surface.configure(
        &renderer_stuff.device,
        &renderer_stuff.surface_configuration,
      );
      let window = renderer_stuff.window_surface.borrow_window();
      renderer_stuff.iced_stuff.viewport = Viewport::with_physical_size(
        iced_core::Size::new(new_inner_size.width, new_inner_size.height),
        window.scale_factor(), //TODO: handle scale factor changes
      );
    }
  }
}

impl Default for WinitRunner {
  fn default() -> Self {
    Self::new()
  }
}

impl ApplicationHandler for WinitRunner {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    event_loop.set_control_flow(ControlFlow::Wait);

    // --- Create the main window ---
    let main_window = self.get_main_window(event_loop);

    if self.window_renderer_stuff.is_none() {
      let rendering_stuff_future = WGPURenderingStuff::new(main_window);
      // TODO: maybe move to background thread
      match block_on(rendering_stuff_future) {
        Ok(mut rendering_stuff) => {
          // Initialize the iced runtime state
          self.iced_state = Some(iced_runtime::program::State::new(
            ShaderToyApp::new(),
            rendering_stuff.iced_stuff.viewport.logical_size(),
            &mut rendering_stuff.iced_stuff.renderer,
            &mut rendering_stuff.iced_stuff.debug,
          ));

          self.window_renderer_stuff = Some(rendering_stuff);
          if let Some(main_window) = &self.window {
            main_window.set_visible(true);
          }
        },
        Err(e) => {
          error!("Failed to initialize rendering stuff: {}", e);
          event_loop.exit();
        },
      }
    }
  }

  fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    self.window_renderer_stuff = None;
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    window_id: WindowId,
    event: WindowEvent,
  ) {
    match event {
      WindowEvent::RedrawRequested => {
        if let Some(renderer_stuff) = &mut self.window_renderer_stuff {
          let WGPURenderingStuff {
            device,
            queue,
            iced_stuff,
            window_surface,
            ..
          } = renderer_stuff;

          let window = window_surface.borrow_window();
          let surface = window_surface.borrow_surface();

          let IcedStuff {
            engine,
            renderer,
            viewport,
            debug,
          } = iced_stuff;

          match surface.get_current_texture() {
            Ok(frame) => {
              let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Iced Commands"),
              });

              let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

              // And then iced on top
              renderer.present(
                engine,
                device,
                queue,
                &mut encoder,
                Some(iced_core::Color::WHITE),
                frame.texture.format(),
                &view,
                viewport,
                &debug.overlay(),
              );

              // Then we submit the work
              engine.submit(queue, encoder);
              frame.present();

              // Update the mouse cursor
              if let Some(state) = &mut self.iced_state {
                window.set_cursor(iced_winit::conversion::mouse_interaction(
                  state.mouse_interaction(),
                ));
              }
            },
            Err(error) => match error {
              wgpu::SurfaceError::OutOfMemory => {
                panic!("Swapchain error: {error}. Rendering cannot continue.")
              },
              _ => {
                // Try rendering again next frame.
                window.request_redraw();
              },
            },
          }
        }
      },
      WindowEvent::CursorMoved { position, .. } => {
        self.cursor_position = Some(position);
      },
      WindowEvent::ModifiersChanged(new_modifiers) => {
        self.modifiers = new_modifiers.state();
      },
      WindowEvent::Resized(new_size) => {
        self.resize_main_window(new_size);
      },
      WindowEvent::CloseRequested => {
        event_loop.exit();
      },
      _ => {
        warn!("Unhandled window event: {:?}", event);
      },
    }

    if let (Some(iced_state), Some(window)) = (&mut self.iced_state, &self.window) {
      // Map window event to iced event
      if let Some(event) =
        iced_winit::conversion::window_event(event, window.scale_factor(), self.modifiers)
      {
        iced_state.queue_event(event);
      }

      if let (Some(rendering_stuff), Some(clipboard)) =
        (&mut self.window_renderer_stuff, &mut self.window_clipboard)
      {
        let WGPURenderingStuff {
          iced_stuff:
            IcedStuff {
              renderer,
              viewport,
              debug,
              ..
            },
          ..
        } = rendering_stuff;

        // If there are events pending
        if !iced_state.is_queue_empty() {
          // We update iced
          let _ = iced_state.update(
            viewport.logical_size(),
            self
              .cursor_position
              .map(|p| conversion::cursor_position(p, viewport.scale_factor()))
              .map(iced_core::mouse::Cursor::Available)
              .unwrap_or(iced_core::mouse::Cursor::Unavailable),
            renderer,
            &iced_core::Theme::default(),
            &iced_core::renderer::Style {
              text_color: iced_core::Color::WHITE,
            },
            clipboard,
            debug,
          );

          // and request a redraw
          window.request_redraw();
        }
      }
    }
  }
}

struct ShaderToyApp {
  app_start: Instant,
  // UI state
  grid_mode: bool,
  shader_to_show: u32,
}

impl ShaderToyApp {
  #[must_use]
  fn new() -> Self {
    Self {
      app_start: Instant::now(),
      grid_mode: false,
      shader_to_show: 0,
    }
  }

  #[must_use]
  fn display_mode(&self) -> shared_data::DisplayMode {
    if self.grid_mode {
      shared_data::DisplayMode::Grid { _padding: 0 }
    } else {
      shared_data::DisplayMode::SingleShader(self.shader_to_show)
    }
  }
}

impl Default for ShaderToyApp {
  fn default() -> Self {
    Self::new()
  }
}

impl iced_runtime::Program for ShaderToyApp {
  type Renderer = iced_wgpu::Renderer;
  type Theme = iced_core::Theme;
  type Message = Message;

  fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
    Task::none()
  }

  fn view(&self) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
    info!("Rendering ShaderToy UI");
    container(Text::new("ShaderToy - Rust GPU")).into()
  }
}

fn new_argparser() -> clap::Command {
  command!().about("ShaderToy - Rust GPU").arg(
    Arg::new("log-level")
      .long("log-level")
      .help("Log level")
      .value_parser(["TRACE", "DEBUG", "INFO", "WARNING", "ERROR"]),
  )
}

fn main() -> Result<(), winit::error::EventLoopError> {
  let matches = new_argparser().get_matches();

  let log_level = matches
    .get_one::<String>("log-level")
    .and_then(|level| tracing::Level::from_str(level).ok());
  let logging_builder = tracing_subscriber::fmt::fmt().with_writer(io::stdout);
  if let Some(level) = log_level {
    logging_builder.with_max_level(level).init();
  } else {
    logging_builder
      .with_env_filter(
        EnvFilter::builder()
          .with_default_directive(LevelFilter::INFO.into())
          .from_env_lossy(),
      )
      .init();
  }

  info!("Starting ShaderToy - Rust GPU");

  // initialize the winit event loop
  let event_loop = EventLoop::new()?;
  let mut app = WinitRunner::default();
  event_loop.run_app(&mut app)
}
