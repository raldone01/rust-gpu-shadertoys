pub struct ShaderRendererWidget<Message, Theme = iced_core::Theme> {
  message_: PhantomData<Message>,
  theme_: PhantomData<Theme>,
}

impl<Message, Theme> Widget<Message, Theme, iced_wgpu::Renderer>
  for ShaderRendererWidget<Message, Theme>
{
  fn size(&self) -> iced_core::Size<iced_core::Length> {
    todo!()
  }

  fn layout(
    &self,
    tree: &mut iced_core::widget::Tree,
    renderer: &iced_wgpu::Renderer,
    limits: &iced_core::layout::Limits,
  ) -> iced_core::layout::Node {
    todo!()
  }

  fn draw(
    &self,
    tree: &iced_core::widget::Tree,
    renderer: &mut iced_wgpu::Renderer,
    theme: &Theme,
    style: &iced_core::renderer::Style,
    layout: iced_core::Layout<'_>,
    cursor: iced_core::mouse::Cursor,
    viewport: &iced_core::Rectangle,
  ) {
    todo!()
  }
}
