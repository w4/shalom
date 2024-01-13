//! Adapted from <https://docs.rs/iced_aw/latest/src/iced_aw/native/overlay/floating_element.rs.html>

use iced::{
    advanced,
    advanced::{
        layout,
        layout::{Limits, Node},
        mouse::{self, Cursor},
        overlay, renderer,
        widget::{Operation, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event, Element, Event, Length, Point, Rectangle, Size, Vector,
};

/// A floating element floating over some content.
///
/// # Example
/// ```ignore
/// # use iced::widget::{button, Button, Column, Text};
/// # use iced_aw::native::FloatingElement;
/// #
/// #[derive(Debug, Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// let content = Column::new();
/// let floating_element = FloatingElement::new(
///     content,
///     || Button::new(Text::new("Press Me!"))
///         .on_press(Message::ButtonPressed)
///         .into()
/// );
/// ```
#[allow(missing_debug_implementations)]
pub struct FloatingElement<'a, Message, Renderer = crate::Renderer>
where
    Renderer: advanced::Renderer,
{
    /// The anchor of the element.
    anchor: Anchor,
    /// The offset of the element.
    offset: Offset,
    /// The visibility of the element.
    hidden: bool,
    /// The underlying element.
    underlay: Element<'a, Message, Renderer>,
    /// The floating element of the [`FloatingElementOverlay`](FloatingElementOverlay).
    element: Element<'a, Message, Renderer>,
}

impl<'a, Message, Renderer> FloatingElement<'a, Message, Renderer>
where
    Renderer: advanced::Renderer,
{
    /// Creates a new [`FloatingElement`](FloatingElement) over some content,
    /// showing the given [`Element`].
    ///
    /// It expects:
    ///     * the underlay [`Element`] on which this [`FloatingElement`](FloatingElement) will be
    ///       wrapped around.
    ///     * a function that will lazily create the [`Element`] for the overlay.
    pub fn new<U, B>(underlay: U, element: B) -> Self
    where
        U: Into<Element<'a, Message, Renderer>>,
        B: Into<Element<'a, Message, Renderer>>,
    {
        FloatingElement {
            anchor: Anchor::SouthEast,
            offset: 5.0.into(),
            hidden: false,
            underlay: underlay.into(),
            element: element.into(),
        }
    }

    /// Sets the [`Anchor`](Anchor) of the [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets the [`Offset`](Offset) of the [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn offset<O>(mut self, offset: O) -> Self
    where
        O: Into<Offset>,
    {
        self.offset = offset.into();
        self
    }

    /// Hide or unhide the [`Element`] on the [`FloatingElement`](FloatingElement).
    #[must_use]
    pub fn hide(mut self, hide: bool) -> Self {
        self.hidden = hide;
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for FloatingElement<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: advanced::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.underlay), Tree::new(&self.element)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.underlay, &self.element]);
    }

    fn size(&self) -> Size<Length> {
        self.underlay.as_widget().size()
    }

    fn layout(&self, state: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.underlay
            .as_widget()
            .layout(&mut state.children[0], renderer, limits)
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.underlay.as_widget_mut().on_event(
            &mut state.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.underlay.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.underlay.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate<'b>(
        &'b self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        self.underlay
            .as_widget()
            .operate(&mut state.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        let [first, second] = &mut state.children[..] else {
            panic!();
        };

        let bounds = layout.bounds();

        let mut group = overlay::Group::new();

        if let Some(overlay) = self
            .underlay
            .as_widget_mut()
            .overlay(first, layout, renderer)
        {
            group = group.push(overlay);
        }

        if !self.hidden {
            group = group.push(overlay::Element::new(
                bounds.position(),
                Box::new(FloatingElementOverlay::new(
                    second,
                    &mut self.element,
                    &self.anchor,
                    &self.offset,
                    bounds,
                )),
            ));
        }

        Some(group.into())
    }
}

impl<'a, Message, Renderer> From<FloatingElement<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: 'a + advanced::Renderer,
{
    fn from(floating_element: FloatingElement<'a, Message, Renderer>) -> Self {
        Element::new(floating_element)
    }
}

/// The internal overlay of a [`FloatingElement`](crate::FloatingElement) for
/// rendering a [`Element`](iced_widget::core::Element) as an overlay.
#[allow(missing_debug_implementations, clippy::module_name_repetitions)]
pub struct FloatingElementOverlay<'a, 'b, Message, Renderer: advanced::Renderer> {
    /// The state of the element.
    state: &'b mut Tree,
    /// The floating element
    element: &'b mut Element<'a, Message, Renderer>,
    /// The anchor of the element.
    anchor: &'b Anchor,
    /// The offset of the element.
    offset: &'b Offset,
    /// The bounds of the underlay element.
    underlay_bounds: Rectangle,
}

impl<'a, 'b, Message, Renderer> FloatingElementOverlay<'a, 'b, Message, Renderer>
where
    Renderer: advanced::Renderer,
{
    /// Creates a new [`FloatingElementOverlay`] containing the given
    /// [`Element`](iced_widget::core::Element).
    pub fn new(
        state: &'b mut Tree,
        element: &'b mut Element<'a, Message, Renderer>,
        anchor: &'b Anchor,
        offset: &'b Offset,
        underlay_bounds: Rectangle,
    ) -> Self {
        FloatingElementOverlay {
            state,
            element,
            anchor,
            offset,
            underlay_bounds,
        }
    }
}

impl<'a, 'b, Message, Renderer> advanced::Overlay<Message, Renderer>
    for FloatingElementOverlay<'a, 'b, Message, Renderer>
where
    Renderer: advanced::Renderer,
{
    fn layout(
        &mut self,
        renderer: &Renderer,
        _bounds: Size,
        position: Point,
        _translation: Vector,
    ) -> layout::Node {
        // Constrain overlay to fit inside the underlay's bounds
        let limits = layout::Limits::new(Size::ZERO, self.underlay_bounds.size())
            .width(Length::Fill)
            .height(Length::Fill);
        let node = self
            .element
            .as_widget()
            .layout(self.state, renderer, &limits);

        let position = match self.anchor {
            Anchor::NorthWest => Point::new(position.x + self.offset.x, position.y + self.offset.y),
            Anchor::NorthEast => Point::new(
                position.x + self.underlay_bounds.width - node.bounds().width - self.offset.x,
                position.y + self.offset.y,
            ),
            Anchor::SouthWest => Point::new(
                position.x + self.offset.x,
                position.y + self.underlay_bounds.height - node.bounds().height - self.offset.y,
            ),
            Anchor::SouthEast => Point::new(
                position.x + self.underlay_bounds.width - node.bounds().width - self.offset.x,
                position.y + self.underlay_bounds.height - node.bounds().height - self.offset.y,
            ),
            Anchor::North => Point::new(
                position.x + self.underlay_bounds.width / 2.0 - node.bounds().width / 2.0
                    + self.offset.x,
                position.y + self.offset.y,
            ),
            Anchor::East => Point::new(
                position.x + self.underlay_bounds.width - node.bounds().width - self.offset.x,
                position.y + self.underlay_bounds.height / 2.0 - node.bounds().height / 2.0
                    + self.offset.y,
            ),
            Anchor::South => Point::new(
                position.x + self.underlay_bounds.width / 2.0 - node.bounds().width / 2.0
                    + self.offset.x,
                position.y + self.underlay_bounds.height - node.bounds().height - self.offset.y,
            ),
            Anchor::West => Point::new(
                position.x + self.offset.x,
                position.y + self.underlay_bounds.height / 2.0 - node.bounds().height / 2.0
                    + self.offset.y,
            ),
        };

        node.move_to(position)
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<Message>,
    ) -> event::Status {
        self.element.as_widget_mut().on_event(
            self.state,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.element
            .as_widget()
            .mouse_interaction(self.state, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        let bounds = layout.bounds();
        self.element
            .as_widget()
            .draw(self.state, renderer, theme, style, layout, cursor, &bounds);
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Renderer>> {
        self.element
            .as_widget_mut()
            .overlay(self.state, layout, renderer)
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum Anchor {
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
    North,
    East,
    South,
    West,
}

#[derive(Copy, Clone, Debug)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

impl From<f32> for Offset {
    fn from(float: f32) -> Self {
        Self { x: float, y: float }
    }
}

impl From<[f32; 2]> for Offset {
    fn from(array: [f32; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

impl From<Offset> for Point {
    fn from(offset: Offset) -> Self {
        Self::new(offset.x, offset.y)
    }
}

impl From<&Offset> for Point {
    fn from(offset: &Offset) -> Self {
        Self::new(offset.x, offset.y)
    }
}
