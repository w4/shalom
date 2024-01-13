#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use iced::{
    alignment::Vertical,
    font::Weight,
    theme::{Container, Svg},
    widget::{component, container, mouse_area, row, svg, text},
    Alignment, Background, Color, Element, Font, Length, Renderer, Theme,
};

use crate::theme::{
    colours::{ORANGE, SYSTEM_GRAY6},
    Icon,
};

pub const LONG_PRESS_LENGTH: Duration = Duration::from_millis(350);

pub fn toggle_card<M>(name: &str, active: bool, disabled: bool) -> ToggleCard<M> {
    ToggleCard {
        name: Box::from(name),
        active,
        disabled,
        ..ToggleCard::default()
    }
}

pub struct ToggleCard<M> {
    icon: Option<Icon>,
    name: Box<str>,
    height: Length,
    width: Length,
    active: bool,
    disabled: bool,
    active_icon_colour: Option<Color>,
    on_press: Option<M>,
    on_long_press: Option<M>,
}

impl<M> Default for ToggleCard<M> {
    fn default() -> Self {
        Self {
            icon: None,
            name: Box::from(""),
            height: Length::Shrink,
            width: Length::Fill,
            active: false,
            disabled: false,
            active_icon_colour: None,
            on_press: None,
            on_long_press: None,
        }
    }
}

impl<M> ToggleCard<M> {
    pub fn active_icon_colour(mut self, color: Option<Color>) -> Self {
        self.active_icon_colour = color;
        self
    }

    pub fn on_press(mut self, msg: M) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn on_long_press(mut self, msg: M) -> Self {
        self.on_long_press = Some(msg);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }
}

impl<M: Clone> iced::widget::Component<M, Renderer> for ToggleCard<M> {
    type State = State;
    type Event = ToggleCardEvent;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            ToggleCardEvent::Down => {
                state.mouse_down_start = Some(Instant::now());

                None
            }
            ToggleCardEvent::Up => {
                let Some(start) = state.mouse_down_start.take() else {
                    return None;
                };

                if start.elapsed() > LONG_PRESS_LENGTH {
                    self.on_long_press.clone().or_else(|| self.on_press.clone())
                } else {
                    self.on_press.clone()
                }
            }
            ToggleCardEvent::Hold => self.on_long_press.clone(),
            ToggleCardEvent::Cancel => {
                state.mouse_down_start = None;

                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let style = match (self.disabled, self.active, state.mouse_down_start) {
            (true, _, _) => Style::Disabled,
            (_, true, None) => Style::Active(self.active_icon_colour),
            (_, true, Some(_)) => Style::ActiveHover(self.active_icon_colour),
            (_, false, None) => Style::Inactive,
            (_, false, Some(_)) => Style::InactiveHover,
        };

        let icon = self.icon.map(|icon| {
            svg(icon)
                .height(28)
                .width(28)
                .style(Svg::Custom(Box::new(style)))
        });

        let name = text(&self.name).size(18).font(Font {
            weight: Weight::Bold,
            // stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let row = if let Some(icon) = icon {
            row![icon, name]
        } else {
            row![name]
        };

        mouse_area(
            container(
                row.spacing(5)
                    .width(self.width)
                    .align_items(Alignment::Center),
            )
            .height(self.height)
            .width(self.width)
            .style(Container::Custom(Box::new(style)))
            .align_y(Vertical::Bottom)
            .padding([20, 20]),
        )
        .on_press(ToggleCardEvent::Down)
        .on_release(ToggleCardEvent::Up)
        .on_hold(ToggleCardEvent::Hold, LONG_PRESS_LENGTH)
        .on_cancel(ToggleCardEvent::Cancel)
        .into()
    }
}

impl<'a, M> From<ToggleCard<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: ToggleCard<M>) -> Self {
        component(card)
    }
}

#[derive(Default)]
pub struct State {
    mouse_down_start: Option<Instant>,
}

#[derive(Clone)]
pub enum ToggleCardEvent {
    Down,
    Up,
    Hold,
    Cancel,
}

#[derive(Copy, Clone)]
pub enum Style {
    Active(Option<Color>),
    ActiveHover(Option<Color>),
    Inactive,
    InactiveHover,
    Disabled,
}

impl container::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let base = container::Appearance {
            text_color: None,
            background: None,
            border_radius: 10.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        };

        match self {
            Style::Disabled => container::Appearance {
                text_color: Some(Color {
                    a: 0.6,
                    ..Color::WHITE
                }),
                background: Some(Background::Color(Color {
                    a: 0.9,
                    ..SYSTEM_GRAY6
                })),
                ..base
            },
            Style::Inactive => container::Appearance {
                text_color: Some(Color {
                    a: 0.7,
                    ..Color::WHITE
                }),
                background: Some(Background::Color(Color {
                    a: 0.7,
                    ..SYSTEM_GRAY6
                })),
                ..base
            },
            Style::InactiveHover => container::Appearance {
                text_color: Some(Color {
                    a: 0.7,
                    ..Color::WHITE
                }),
                background: Some(Background::Color(Color {
                    a: 0.9,
                    ..SYSTEM_GRAY6
                })),
                ..base
            },
            Style::Active(_) => container::Appearance {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(Color {
                    a: 0.8,
                    ..Color::WHITE
                })),
                ..base
            },
            Style::ActiveHover(_) => container::Appearance {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(Color {
                    a: 0.6,
                    ..Color::WHITE
                })),
                ..base
            },
        }
    }
}

impl svg::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> svg::Appearance {
        let base = <Self as container::StyleSheet>::appearance(self, style)
            .text_color
            .unwrap_or(Color::WHITE);

        match self {
            Style::Active(_) | Style::ActiveHover(_) => svg::Appearance {
                color: Some(Color {
                    a: base.a,
                    ..ORANGE
                }),
            },
            Style::Inactive | Style::InactiveHover | Style::Disabled => {
                svg::Appearance { color: Some(base) }
            }
        }
    }
}
