use std::time::Instant;

use iced::{
    advanced::{
        graphics::core::Element,
        layout::{Limits, Node},
        mouse,
        renderer::{Quad, Style},
        widget::{tree::Tag, Tree},
        Clipboard, Layout, Renderer as IRenderer, Shell, Widget,
    },
    event::Status,
    mouse::{Cursor, Interaction},
    widget::{
        text_input::{Appearance, Id},
        Text,
    },
    window::RedrawRequest,
    Alignment, Background, Color, Length, Rectangle, Renderer, Size, Theme, Vector,
};
use keyframe::{functions::EaseOutQuint, keyframes, AnimationSequence};

use crate::theme::Icon;

// text height
const INITIAL_SEARCH_BOX_SIZE: Size = Size::new(78., 78.);

pub fn header_search<'a, M>(
    on_input: fn(String) -> M,
    open_state_toggle: M,
    open: bool,
    search_query: &str,
    mut header: Text<'a, Renderer>,
    dy_mult: f32,
) -> HeaderSearch<'a, M>
where
    M: Clone + 'a,
{
    if open {
        header = header.style(iced::theme::Text::Color(Color {
            a: 0.0,
            ..Color::WHITE
        }));
    }

    HeaderSearch {
        original_header: header.clone(),
        header,
        open,
        open_state_toggle,
        input: iced::widget::text_input("Search...", search_query)
            .id(Id::unique())
            .on_input(on_input)
            .style(iced::theme::TextInput::Custom(Box::new(InputStyle)))
            .into(),
        search_icon: Element::from(Icon::Search.canvas(Color::BLACK)),
        close_icon: Element::from(Icon::Close.canvas(Color::BLACK)),
        dy_mult,
    }
}

pub struct HeaderSearch<'a, M> {
    original_header: Text<'a, Renderer>,
    header: Text<'a, Renderer>,
    input: Element<'a, M, Renderer>,
    search_icon: Element<'a, M, Renderer>,
    close_icon: Element<'a, M, Renderer>,
    dy_mult: f32,
    open: bool,
    open_state_toggle: M,
}

impl<'a, M> Widget<M, Renderer> for HeaderSearch<'a, M>
where
    M: Clone + 'a,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let text_node = <iced::advanced::widget::Text<'_, Renderer> as Widget<M, Renderer>>::layout(
            &self.original_header,
            &mut tree.children[0],
            renderer,
            limits,
        );

        let size = limits.height(Length::Fixed(text_node.size().height)).max();

        let state = tree.state.downcast_mut::<State>();
        match (&state, self.open) {
            (State::Open, false) => {
                *state = State::close();
            }
            (State::Closed, true) => {
                *state = State::open();
            }
            _ => {}
        }

        let current_search_box_size = match &state {
            State::Animate {
                search_box_size, ..
            } => Size {
                width: INITIAL_SEARCH_BOX_SIZE.width
                    + ((size.width - INITIAL_SEARCH_BOX_SIZE.width) * search_box_size.now()),
                ..INITIAL_SEARCH_BOX_SIZE
            },
            State::Closed => INITIAL_SEARCH_BOX_SIZE,
            State::Open => Size {
                width: limits.max().width,
                ..INITIAL_SEARCH_BOX_SIZE
            },
        };

        let search_icon_size = Size::new(36., 36.);
        let search_icon_node = Node::new(search_icon_size)
            .translate(Vector {
                x: -(INITIAL_SEARCH_BOX_SIZE.width - search_icon_size.width) / 2.0,
                y: 0.0,
            })
            .align(Alignment::End, Alignment::Center, current_search_box_size);

        let search_input = self
            .input
            .as_widget()
            .layout(
                &mut tree.children[1],
                renderer,
                &limits.width(current_search_box_size.width).shrink([80, 20]),
            )
            .translate(Vector { x: 20.0, y: 0.0 })
            .align(Alignment::Start, Alignment::Center, current_search_box_size);

        let search_box = Node::with_children(
            current_search_box_size,
            vec![search_icon_node, search_input],
        )
        .align(Alignment::End, Alignment::Center, size);

        Node::with_children(size, vec![text_node, search_box])
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
        let local_state = state.state.downcast_ref::<State>();
        let mut layout_children = layout.children();
        let text_layout = layout_children.next().unwrap();
        let search_layout = layout_children.next().unwrap();
        let mut search_children = search_layout.children();

        <iced::advanced::widget::Text<'_, Renderer> as Widget<M, Renderer>>::draw(
            &self.header,
            &state.children[0],
            renderer,
            theme,
            style,
            text_layout,
            cursor,
            viewport,
        );

        let border_radius = if matches!(local_state, State::Open) {
            20.0 * (1.0 - self.dy_mult)
        } else {
            20.0
        };

        renderer.fill_quad(
            Quad {
                bounds: search_layout.bounds(),
                border_radius: border_radius.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Background::Color(Color::WHITE),
        );

        let icon_bounds = search_children.next().unwrap();

        if !matches!(local_state, State::Open) {
            self.search_icon.as_widget().draw(
                &state.children[2],
                renderer,
                theme,
                style,
                icon_bounds,
                cursor,
                viewport,
            );
        }

        if !matches!(local_state, State::Closed) {
            self.close_icon.as_widget().draw(
                &state.children[3],
                renderer,
                theme,
                style,
                icon_bounds,
                cursor,
                viewport,
            );
        }

        if !matches!(local_state, State::Closed) {
            self.input.as_widget().draw(
                &state.children[1],
                renderer,
                theme,
                style,
                search_children.next().unwrap(),
                cursor,
                viewport,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn on_event(
        &mut self,
        tree_state: &mut Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
        let state = tree_state.state.downcast_mut::<State>();

        match state {
            State::Open if !self.open => {
                *state = State::close();
                shell.request_redraw(RedrawRequest::NextFrame);
            }
            State::Closed if self.open => {
                *state = State::open();
                shell.request_redraw(RedrawRequest::NextFrame);
            }
            _ => {}
        }

        let status = match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(mouse::Button::Left))
            | iced::Event::Touch(iced::touch::Event::FingerPressed { .. })
                if cursor.is_over(
                    layout
                        .children()
                        .nth(1)
                        .unwrap()
                        .children()
                        .next()
                        .unwrap()
                        .bounds(),
                ) =>
            {
                if !matches!(state, State::Animate { .. }) {
                    shell.publish(self.open_state_toggle.clone());
                }

                Status::Captured
            }
            iced::Event::Window(_, iced::window::Event::RedrawRequested(_)) => {
                let State::Animate {
                    last_draw,
                    next_state,
                    text_opacity,
                    search_box_size,
                    search_icon,
                    close_icon,
                } = state
                else {
                    return Status::Ignored;
                };

                let elapsed = last_draw.elapsed().as_secs_f64();
                *last_draw = Instant::now();

                text_opacity.advance_by(elapsed);
                self.header = self
                    .header
                    .clone()
                    .style(iced::theme::Text::Color(Color {
                        a: text_opacity.now(),
                        ..Color::WHITE
                    }))
                    .size(60.0 - (2.0 * (1.0 - text_opacity.now())));

                search_box_size.advance_by(elapsed);

                search_icon.advance_by(elapsed);
                self.search_icon = Element::from(Icon::Search.canvas(Color {
                    a: search_icon.now(),
                    ..Color::BLACK
                }));

                close_icon.advance_by(elapsed);
                self.close_icon = Element::from(Icon::Close.canvas(Color {
                    a: close_icon.now(),
                    ..Color::BLACK
                }));

                if text_opacity.finished() && search_box_size.finished() {
                    *state = std::mem::take(next_state);
                }

                shell.request_redraw(RedrawRequest::NextFrame);

                Status::Captured
            }
            _ => Status::Ignored,
        };

        if status == Status::Ignored {
            self.input.as_widget_mut().on_event(
                &mut tree_state.children[1],
                event,
                layout.children().nth(1).unwrap().children().nth(1).unwrap(),
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            )
        } else {
            status
        }
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> Interaction {
        self.input.as_widget().mouse_interaction(
            &state.children[1],
            layout.children().nth(1).unwrap().children().nth(1).unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(if self.open {
            State::Open
        } else {
            State::Closed
        })
    }

    fn tag(&self) -> Tag {
        Tag::of::<State>()
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&Element::<'_, M, _>::from(self.header.clone())),
            Tree::new(&self.input),
            Tree::new(&self.search_icon),
            Tree::new(&self.close_icon),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[
            &Element::from(self.header.clone()),
            &self.input,
            &self.search_icon,
            &self.close_icon,
        ]);
    }
}

#[derive(Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub enum State {
    #[default]
    Closed,
    Animate {
        last_draw: Instant,
        next_state: Box<State>,
        text_opacity: AnimationSequence<f32>,
        search_box_size: AnimationSequence<f32>,
        search_icon: AnimationSequence<f32>,
        close_icon: AnimationSequence<f32>,
    },
    Open,
}

impl State {
    fn open() -> Self {
        Self::Animate {
            last_draw: Instant::now(),
            next_state: Box::new(State::Open),
            text_opacity: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
            search_box_size: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
            search_icon: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
            close_icon: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
        }
    }

    fn close() -> Self {
        Self::Animate {
            last_draw: Instant::now(),
            next_state: Box::new(State::Closed),
            text_opacity: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
            search_box_size: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
            search_icon: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
            close_icon: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
        }
    }
}

impl<'a, M> From<HeaderSearch<'a, M>> for iced::Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(modal: HeaderSearch<'a, M>) -> Self {
        iced::Element::new(modal)
    }
}

pub struct InputStyle;

impl iced::widget::text_input::StyleSheet for InputStyle {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> Appearance {
        Appearance {
            background: Background::Color(Color::WHITE),
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::default(),
            icon_color: Color::default(),
        }
    }

    fn focused(&self, style: &Self::Style) -> Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        let palette = style.extended_palette();

        palette.background.strong.color
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        let palette = style.extended_palette();

        palette.background.base.text
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        self.placeholder_color(style)
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        let palette = style.extended_palette();

        palette.primary.weak.color
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        self.active(style)
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        self.active(style)
    }
}
