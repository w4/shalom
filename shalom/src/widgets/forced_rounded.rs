use iced::{
    advanced::{
        layout::{Limits, Node},
        overlay,
        overlay::Element,
        renderer::{Quad, Style},
        widget::{
            tree::{State, Tag},
            Operation, Tree,
        },
        Clipboard, Layout, Renderer, Shell, Widget,
    },
    event::Status,
    mouse::{Cursor, Interaction},
    Background, Color, Event, Length, Point, Rectangle, Size, Vector,
};

pub fn forced_rounded<'a, M: 'a, R>(
    element: impl Into<iced::Element<'a, M, R>>,
) -> ForcedRounded<'a, M, R> {
    ForcedRounded {
        element: element.into(),
    }
}

pub struct ForcedRounded<'a, M, R> {
    element: iced::Element<'a, M, R>,
}

impl<'a, M, R: Renderer> Widget<M, R> for ForcedRounded<'a, M, R> {
    fn size(&self) -> Size<Length> {
        self.element.as_widget().size()
    }

    fn layout(&self, tree: &mut Tree, renderer: &R, limits: &Limits) -> Node {
        self.element.as_widget().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut R,
        theme: &R::Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.element
            .as_widget()
            .draw(state, renderer, theme, style, layout, cursor, viewport);
    }

    fn tag(&self) -> Tag {
        self.element.as_widget().tag()
    }

    fn children(&self) -> Vec<Tree> {
        self.element.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.element.as_widget().diff(tree);
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &R,
    ) -> Interaction {
        self.element
            .as_widget()
            .mouse_interaction(state, layout, cursor, viewport, renderer)
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        self.element.as_widget_mut().on_event(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
        )
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &R,
        operation: &mut dyn Operation<M>,
    ) {
        self.element
            .as_widget()
            .operate(state, layout, renderer, operation);
    }

    fn state(&self) -> State {
        self.element.as_widget().state()
    }

    fn overlay<'b>(
        &'b mut self,
        _state: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &R,
    ) -> Option<Element<'b, M, R>> {
        Some(overlay::Element::new(
            layout.position(),
            Box::new(Overlay {
                size: layout.bounds().size(),
                position: Some(layout.bounds().position()),
            }),
        ))
    }
}

impl<'a, M, R> From<ForcedRounded<'a, M, R>> for iced::Element<'a, M, R>
where
    M: 'a + Clone,
    R: 'a + Renderer,
{
    fn from(e: ForcedRounded<'a, M, R>) -> Self {
        iced::Element::new(e)
    }
}

pub struct Overlay {
    pub size: Size,
    pub position: Option<Point>,
}

impl<M, R: Renderer> overlay::Overlay<M, R> for Overlay {
    fn layout(
        &mut self,
        _renderer: &R,
        _bounds: Size,
        position: Point,
        _translation: Vector,
    ) -> Node {
        Node::new(self.size).move_to(self.position.unwrap_or(position))
    }

    fn draw(
        &self,
        renderer: &mut R,
        _theme: &R::Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
    ) {
        renderer.with_layer(layout.bounds(), |renderer| {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        height: self.size.height + 12.,
                        width: self.size.width + 12.,
                        x: layout.bounds().x - 6.,
                        y: layout.bounds().y - 6.,
                    },
                    border_radius: [20., 20., 20., 20.].into(),
                    border_width: 10.,
                    border_color: Color::WHITE,
                },
                Background::Color(Color::TRANSPARENT),
            );
        });
    }

    fn is_over(&self, _layout: Layout<'_>, _renderer: &R, _cursor_position: Point) -> bool {
        false
    }
}
