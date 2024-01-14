use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

use iced::{
    advanced::{
        layout::{Limits, Node},
        overlay, renderer,
        renderer::Style,
        widget::{tree, tree::Tag, Operation, Tree},
        Clipboard, Layout, Renderer as RendererT, Shell, Widget,
    },
    event::Status,
    mouse,
    mouse::{Button, Cursor, Interaction},
    window,
    window::RedrawRequest,
    Alignment, BorderRadius, Color, Element, Event, Length, Point, Rectangle, Renderer, Size,
    Theme, Vector,
};
use keyframe::{functions::EaseOutQuint, keyframes, AnimationSequence};

use super::blackhole_event::blackhole_event;

pub struct ContextMenu<'a, M> {
    base: Element<'a, M, Renderer>,
    content: Element<'a, M, Renderer>,
    on_close: Option<M>,
    max_height: f32,
}

impl<'a, M> ContextMenu<'a, M> {
    pub fn new(
        base: impl Into<Element<'a, M, Renderer>>,
        content: impl Into<Element<'a, M, Renderer>>,
    ) -> Self {
        Self {
            base: base.into(),
            content: content.into(),
            on_close: None,
            max_height: 400.,
        }
    }

    pub fn on_close(mut self, msg: M) -> Self {
        self.on_close = Some(msg);
        self
    }
}

impl<'a, M: Clone> Widget<M, Renderer> for ContextMenu<'a, M> {
    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.base
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn tag(&self) -> Tag {
        Tag::of::<OverlayState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(OverlayState {
            max_height: self.max_height,
            height: 0.,
            state: State::Animate(Instant::now(), keyframes![(0.0, 0.0), (400.0, 0.7)]),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.base), Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.base, &self.content]);
    }

    fn operate(
        &self,
        state: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<M>,
    ) {
        self.base
            .as_widget()
            .operate(state, layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        self.base.as_widget_mut().on_event(
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

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, M, Renderer>> {
        let [ref mut base_tree, ref mut content_tree] = &mut state.children[..] else {
            panic!();
        };

        let mut group = overlay::Group::new();

        if let Some(child) = self
            .base
            .as_widget_mut()
            .overlay(base_tree, layout, renderer)
        {
            group = group.push(overlay::Element::new(
                layout.position(),
                Box::new(blackhole_event(child)),
            ));
        }

        Some(
            group
                .push(overlay::Element::new(
                    layout.position(),
                    Box::new(Overlay {
                        content: &mut self.content,
                        tree: content_tree,
                        on_close: self.on_close.clone(),
                        state: state.state.downcast_mut::<OverlayState>(),
                    }),
                ))
                .into(),
        )
    }
}

struct Overlay<'a, 'b, M> {
    content: &'b mut Element<'a, M, Renderer>,
    tree: &'b mut Tree,
    on_close: Option<M>,
    state: &'b mut OverlayState,
}

impl<'a, 'b, M: Clone> Overlay<'a, 'b, M> {
    pub fn handle_mouse_event(
        &mut self,
        event: &mouse::Event,
        layout: &Layout<'_>,
        cursor: Cursor,
    ) -> bool {
        if matches!(self.state.state, State::Closed) {
            return false;
        }

        match (&self.state.state, event) {
            (State::Dragging(_drag_start, _last_position), mouse::Event::CursorMoved { .. }) => {
                true
            }
            (_, mouse::Event::ButtonPressed(Button::Left)) => {
                self.state.state =
                    State::Dragging(Instant::now(), cursor.position().unwrap_or_default());
                true
            }
            (
                State::Dragging(drag_start, _last_position),
                mouse::Event::ButtonReleased(Button::Left),
            ) => {
                if drag_start.elapsed() <= Duration::from_millis(100)
                    && !cursor.is_over(layout.children().next().unwrap().bounds())
                {
                    // assume all fast clicks outside of content is an intent to
                    // close
                    self.state.state = State::Animate(
                        Instant::now(),
                        keyframes![(self.state.height, 0.0, EaseOutQuint), (0.0, 0.75)],
                    );
                } else if self.state.height <= 0. {
                    // dragged all the way closed, no need to animate anything
                    self.state.state = State::Closed;
                } else if self.state.height < (self.state.max_height / 1.25) {
                    // snap height reached for closing
                    self.state.state = State::Animate(
                        Instant::now(),
                        keyframes![
                            (self.state.height, 0.0, EaseOutQuint),
                            (
                                0.0,
                                (0.75 * (self.state.height.abs() / self.state.max_height.abs()))
                            )
                        ],
                    );
                } else {
                    // snap back to max height
                    self.state.state = State::Animate(
                        Instant::now(),
                        keyframes![
                            (self.state.height, 0.0, EaseOutQuint),
                            (
                                self.state.max_height,
                                (0.75 * (self.state.height.abs() / self.state.max_height.abs()))
                            )
                        ],
                    );
                }

                true
            }
            _ => false,
        }
    }

    pub fn handle_redraw(&mut self, shell: &mut Shell<M>, cursor: Cursor) {
        match &mut self.state.state {
            State::Open | State::Closed => {
                // don't need to do anything here
            }
            State::Dragging(_, last_position) => {
                if let Some(current_position) = cursor.position() {
                    let dy = current_position.y - last_position.y;

                    match dy.total_cmp(&0.) {
                        Ordering::Greater => {
                            self.state.height -= dy;
                            self.state.height = self.state.height.max(0.);
                        }
                        Ordering::Less if self.state.height > self.state.max_height => {
                            self.state.height -=
                                dy / (10. * (self.state.height / self.state.max_height));
                        }
                        Ordering::Less => {
                            self.state.height -= dy;
                        }
                        Ordering::Equal => {}
                    }

                    *last_position = current_position;
                }
            }
            State::Animate(instant, keyframes) => {
                keyframes.advance_by(instant.elapsed().as_secs_f64());
                self.state.height = keyframes.now();
                *instant = Instant::now();

                if keyframes.finished() {
                    if self.state.height <= 0. {
                        if let Some(event) = self.on_close.clone() {
                            shell.publish(event);
                        }

                        self.state.state = State::Closed;
                    } else {
                        self.state.state = State::Open;
                    }
                } else {
                    shell.request_redraw(RedrawRequest::NextFrame);
                }
            }
        }
    }
}

impl<'a, 'b, M: Clone> overlay::Overlay<M, Renderer> for Overlay<'a, 'b, M> {
    fn layout(
        &mut self,
        renderer: &Renderer,
        bounds: Size,
        position: Point,
        _translation: Vector,
    ) -> Node {
        let limits = Limits::new(Size::ZERO, bounds)
            .width(Length::Fill)
            .height(Length::Fill);

        let child = self
            .content
            .as_widget()
            .layout(self.tree, renderer, &limits)
            .align(Alignment::Start, Alignment::Start, limits.max())
            .move_to(Point {
                x: 0.0,
                y: bounds.height - self.state.height,
            });

        Node::with_children(bounds, vec![child]).move_to(position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        if matches!(self.state.state, State::Closed) {
            return;
        }

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border_radius: BorderRadius::default(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            Color {
                a: (0.8 / (self.state.max_height / self.state.height)).min(0.9),
                ..Color::BLACK
            },
        );

        let bounds = Rectangle::new(
            Point::new(0.0, layout.bounds().height - self.state.height),
            Size {
                width: layout.bounds().width,
                height: self.state.height,
            },
        );

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Color::WHITE,
        );

        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            &bounds,
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<M>,
    ) {
        self.content.as_widget().operate(
            self.tree,
            layout.children().next().unwrap(),
            renderer,
            operation,
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) -> Status {
        let mut status = self.content.as_widget_mut().on_event(
            self.tree,
            event.clone(),
            layout.children().next().unwrap(),
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );

        if let (Status::Ignored, Event::Mouse(mouse_event)) = (status, &event) {
            let captured = self.handle_mouse_event(mouse_event, &layout, cursor);

            if captured {
                status = Status::Captured;

                if let (Some(msg), State::Closed) = (&self.on_close, &self.state.state) {
                    shell.publish(msg.clone());
                } else {
                    shell.request_redraw(RedrawRequest::NextFrame);
                }
            }
        } else if let Event::Window(_, window::Event::RedrawRequested(_)) = &event {
            self.handle_redraw(shell, cursor);
        }

        status
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, M, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(self.tree, layout.children().next().unwrap(), renderer)
    }
}

impl<'a, M> From<ContextMenu<'a, M>> for Element<'a, M>
where
    M: 'a + Clone,
{
    fn from(modal: ContextMenu<'a, M>) -> Self {
        Element::new(modal)
    }
}

#[derive(Default)]
pub struct OverlayState {
    max_height: f32,
    height: f32,
    state: State,
}

#[derive(Default)]
pub enum State {
    #[default]
    Open,
    Animate(Instant, AnimationSequence<f32>),
    Dragging(Instant, Point),
    Closed,
}
