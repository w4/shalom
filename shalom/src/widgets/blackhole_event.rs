//! Prevents any events from reaching the inner element.

use iced::{advanced::overlay, Vector};

pub fn blackhole_event<O>(o: O) -> BlackholeEvent<O> {
    BlackholeEvent { overlay: o }
}

pub struct BlackholeEvent<O> {
    overlay: O,
}

impl<Message, Renderer: iced::advanced::Renderer> overlay::Overlay<Message, Renderer>
    for BlackholeEvent<overlay::Element<'_, Message, Renderer>>
{
    fn operate(
        &mut self,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation<Message>,
    ) {
        self.overlay.operate(layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        _event: iced::Event,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        _shell: &mut iced::advanced::Shell<'_, Message>,
    ) -> iced::event::Status {
        iced::event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        iced::advanced::mouse::Interaction::Idle
    }

    fn is_over(
        &self,
        _layout: iced::advanced::Layout<'_>,
        _renderer: &Renderer,
        _cursor_position: iced::Point,
    ) -> bool {
        false
    }

    fn overlay<'a>(
        &'a mut self,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'a, Message, Renderer>> {
        self.overlay.overlay(layout, renderer)
    }

    fn layout(
        &self,
        renderer: &Renderer,
        bounds: iced::Size,
        _position: iced::Point,
    ) -> iced::advanced::layout::Node {
        self.overlay.layout(renderer, bounds, Vector::ZERO)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &<Renderer as iced::advanced::Renderer>::Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
    ) {
        self.overlay.draw(renderer, theme, style, layout, cursor);
    }
}
