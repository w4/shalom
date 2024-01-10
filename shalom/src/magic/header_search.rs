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

const INITIAL_SEARCH_BOX_SIZE: Size = Size::new(54., 54.);

pub fn header_search<'a, M>(
    on_input: fn(String) -> M,
    on_state_change: fn(bool) -> M,
    open: bool,
    search_query: &str,
    mut header: Text<'a, Renderer>,
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

    let current_search_box_size = if open { BoxSize::Fill } else { BoxSize::Min };

    HeaderSearch {
        header,
        current_search_box_size,
        input: iced::widget::text_input("Search...", search_query)
            .id(Id::unique())
            .on_input(on_input)
            .style(iced::theme::TextInput::Custom(Box::new(InputStyle)))
            .into(),
        on_state_change,
        search_icon: Element::from(Icon::Search.canvas(Color::BLACK)),
        close_icon: Element::from(Icon::Close.canvas(Color::BLACK)),
    }
}

#[derive(Debug)]
pub enum BoxSize {
    Fill,
    Min,
    Fixed(Size),
}

pub struct HeaderSearch<'a, M> {
    header: Text<'a, Renderer>,
    current_search_box_size: BoxSize,
    on_state_change: fn(bool) -> M,
    input: Element<'a, M, Renderer>,
    search_icon: Element<'a, M, Renderer>,
    close_icon: Element<'a, M, Renderer>,
}

impl<'a, M> Widget<M, Renderer> for HeaderSearch<'a, M>
where
    M: Clone + 'a,
{
    fn width(&self) -> Length {
        Length::Fill
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let text_node = <iced::advanced::widget::Text<'_, Renderer> as Widget<M, Renderer>>::layout(
            &self.header,
            renderer,
            limits,
        );

        let size = limits.height(Length::Fixed(text_node.size().height)).max();

        let current_search_box_size = match self.current_search_box_size {
            BoxSize::Fixed(size) => size,
            BoxSize::Min => INITIAL_SEARCH_BOX_SIZE,
            BoxSize::Fill => Size {
                width: limits.max().width,
                ..INITIAL_SEARCH_BOX_SIZE
            },
        };

        let search_icon_size = Size::new(36., 36.);
        let mut search_icon_node = Node::new(search_icon_size).translate(Vector {
            x: -(INITIAL_SEARCH_BOX_SIZE.width - search_icon_size.width) / 2.0,
            y: 0.0,
        });
        search_icon_node.align(Alignment::End, Alignment::Center, current_search_box_size);

        let mut search_input = self
            .input
            .as_widget()
            .layout(
                renderer,
                &limits
                    .width(current_search_box_size.width)
                    .pad([0, 20, 0, 60].into()),
            )
            .translate(Vector { x: 20.0, y: 0.0 });
        search_input.align(Alignment::Start, Alignment::Center, current_search_box_size);

        let mut search_box = Node::with_children(
            current_search_box_size,
            vec![search_icon_node, search_input],
        );
        search_box.align(Alignment::End, Alignment::Center, size);

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
            state,
            renderer,
            theme,
            style,
            text_layout,
            cursor,
            viewport,
        );

        renderer.fill_quad(
            Quad {
                bounds: search_layout.bounds(),
                border_radius: 1000.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Background::Color(Color::WHITE),
        );

        let icon_bounds = search_children.next().unwrap();

        if !matches!(local_state, State::Open) {
            self.search_icon.as_widget().draw(
                &state.children[1],
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
            self.input.as_widget().draw(
                &state.children[0],
                renderer,
                theme,
                style,
                search_children.next().unwrap(),
                cursor,
                viewport,
            );
        }
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) -> Status {
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
                let state = state.state.downcast_mut::<State>();
                *state = state.clone().flip();
                shell.request_redraw(RedrawRequest::NextFrame);
                Status::Captured
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let state = state.state.downcast_mut::<State>();
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
                self.header = self.header.clone().style(iced::theme::Text::Color(Color {
                    a: text_opacity.now(),
                    ..Color::WHITE
                }));

                search_box_size.advance_by(elapsed);
                self.current_search_box_size = BoxSize::Fixed(Size {
                    width: INITIAL_SEARCH_BOX_SIZE.width
                        + ((layout.bounds().width - INITIAL_SEARCH_BOX_SIZE.width)
                            * search_box_size.now()),
                    ..INITIAL_SEARCH_BOX_SIZE
                });

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

                    match &state {
                        State::Open => {
                            shell.publish((self.on_state_change)(true));
                            self.current_search_box_size = BoxSize::Fill;
                        }
                        State::Closed => {
                            shell.publish((self.on_state_change)(false));
                            self.current_search_box_size = BoxSize::Min;
                        }
                        State::Animate { .. } => {}
                    }
                }

                shell.request_redraw(RedrawRequest::NextFrame);

                Status::Captured
            }
            _ => Status::Ignored,
        };

        if status == Status::Ignored {
            self.input.as_widget_mut().on_event(
                &mut state.children[0],
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
            &state.children[0],
            layout.children().nth(1).unwrap().children().nth(1).unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(
            if matches!(self.current_search_box_size, BoxSize::Fill) {
                State::Open
            } else {
                State::Closed
            },
        )
    }

    fn tag(&self) -> Tag {
        Tag::of::<State>()
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.input),
            Tree::new(&self.search_icon),
            Tree::new(&self.close_icon),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.input, &self.search_icon, &self.close_icon]);
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
    fn flip(self) -> Self {
        match self {
            State::Closed => Self::Animate {
                last_draw: Instant::now(),
                next_state: Box::new(State::Open),
                text_opacity: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
                search_box_size: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
                search_icon: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
                close_icon: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
            },
            State::Open => Self::Animate {
                last_draw: Instant::now(),
                next_state: Box::new(State::Closed),
                text_opacity: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
                search_box_size: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
                search_icon: keyframes![(0.0, 0.0, EaseOutQuint), (0.0, 0.1), (1.0, 0.5)],
                close_icon: keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
            },
            v @ State::Animate { .. } => v,
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
