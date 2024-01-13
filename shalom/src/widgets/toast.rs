use std::time::{Duration, Instant};

use iced::{
    advanced::{
        graphics::text::Paragraph,
        layout,
        layout::{Limits, Node},
        renderer::{Quad, Style},
        text::{LineHeight, Shaping},
        widget::{tree::Tag, Tree},
        Clipboard, Layout, Renderer as RendererTrait, Shell, Widget,
    },
    alignment::{Horizontal, Vertical},
    event::Status,
    font::Weight,
    mouse::Cursor,
    widget::{text, text::Appearance},
    window,
    window::RedrawRequest,
    Background, Color, Element, Event, Font, Length, Rectangle, Renderer, Size, Theme,
};
use keyframe::{functions::EaseOutQuint, keyframes, AnimationSequence};

use crate::theme::colours::SYSTEM_GRAY6;

pub struct Toast {
    pub text: String,
    pub start: Instant,
    pub ttl: Duration,
}

#[allow(clippy::module_name_repetitions)]
pub struct ToastElement<'a, M> {
    toast: &'a Toast,
    on_expiry: Option<M>,
}

impl<'a, M: Clone> ToastElement<'a, M> {
    pub fn new(toast: &'a Toast) -> Self {
        Self {
            toast,
            on_expiry: None,
        }
    }

    pub fn on_expiry(mut self, msg: M) -> Self {
        self.on_expiry = Some(msg);
        self
    }

    fn advance_closing_state(&self, shell: &mut Shell<'_, M>, state: &mut State) {
        match &mut state.state {
            TickerState::Closing(last_tick, v) => {
                if v.finished() {
                    if let Some(msg) = self.on_expiry.clone() {
                        shell.publish(msg);
                    }
                    state.state = TickerState::Closed;
                } else {
                    v.advance_by(last_tick.elapsed().as_secs_f64());
                    *last_tick = Instant::now();
                    shell.request_redraw(RedrawRequest::NextFrame);
                }
            }
            TickerState::Ticking => {
                state.state = TickerState::Closing(
                    Instant::now(),
                    keyframes![(1.0, 0.0, EaseOutQuint), (0.0, 0.5)],
                );
                shell.request_redraw(RedrawRequest::NextFrame);
            }
            TickerState::Closed => {}
        }
    }
}

impl<'a, M: Clone> Widget<M, Renderer> for ToastElement<'a, M> {
    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        _viewport: &Rectangle,
    ) -> Status {
        if let Event::Window(_, window::Event::RedrawRequested(_)) = event {
            if self.toast.start.elapsed() <= self.toast.ttl {
                shell.request_redraw(RedrawRequest::NextFrame);
            } else {
                let state = state.state.downcast_mut::<State>();
                self.advance_closing_state(shell, state);
            }

            Status::Captured
        } else {
            Status::Ignored
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let local_state = tree.state.downcast_mut::<State>();

        layout::padded(
            limits,
            self.size().width,
            self.size().height,
            [20, 20, 20, 20],
            |limits| {
                text::layout(
                    &mut local_state.content,
                    renderer,
                    limits,
                    Length::Shrink,
                    Length::Shrink,
                    &self.toast.text,
                    LineHeight::default(),
                    None,
                    Some(Font {
                        weight: Weight::Normal,
                        ..Font::with_name("Helvetica Neue")
                    }),
                    Horizontal::Center,
                    Vertical::Center,
                    Shaping::Basic,
                )
            },
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let local_state = tree.state.downcast_ref::<State>();

        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: 20.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Background::Color(Color {
                a: 0.7 * local_state.state.alpha_mut(),
                ..SYSTEM_GRAY6
            }),
        );

        let remaining_pct = (1.0
            - self.toast.start.elapsed().as_secs_f32() / self.toast.ttl.as_secs_f32())
        .max(0.0);
        if remaining_pct > 0.0 {
            let base = layout.bounds();
            let timeout_bounds = Rectangle {
                x: base.x + 20.0,
                y: base.y + base.height - 2.0,
                width: (base.width - 20.0) * remaining_pct,
                height: 2.0,
            };
            renderer.fill_quad(
                Quad {
                    bounds: timeout_bounds,
                    border_radius: 20.0.into(),
                    border_width: 0.0,
                    border_color: Color::default(),
                },
                Background::Color(Color {
                    a: 0.7,
                    ..Color::WHITE
                }),
            );
        }

        let mut children = layout.children();

        text::draw(
            renderer,
            style,
            children.next().unwrap(),
            &local_state.content,
            Appearance {
                color: Some(Color {
                    a: 1.0 * local_state.state.alpha_mut(),
                    ..Color::WHITE
                }),
            },
            viewport,
        );
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(State::default())
    }

    fn tag(&self) -> Tag {
        Tag::of::<State>()
    }
}

#[derive(Default)]
pub struct State {
    content: text::State<Paragraph>,
    state: TickerState,
}

#[derive(Default)]
pub enum TickerState {
    #[default]
    Ticking,
    Closing(Instant, AnimationSequence<f32>),
    Closed,
}

impl TickerState {
    pub fn alpha_mut(&self) -> f32 {
        match self {
            Self::Ticking => 1.0,
            Self::Closing(_, v) => v.now(),
            Self::Closed => 0.0,
        }
    }
}

impl<'a, M> From<ToastElement<'a, M>> for Element<'a, M>
where
    M: 'a + Clone,
{
    fn from(modal: ToastElement<'a, M>) -> Self {
        Element::new(modal)
    }
}
