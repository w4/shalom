#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use iced::{
    alignment::Vertical,
    font::{Stretch, Weight},
    theme::{Container, Svg},
    widget::{column, component, container, svg, text},
    Alignment, Background, Color, Element, Font, Length, Renderer, Theme,
};

use crate::{
    theme::{
        colours::{AMBER_200, SKY_400, SKY_500, SLATE_200, SLATE_300},
        Icon,
    },
    widgets::mouse_area::mouse_area,
};

pub const LONG_PRESS_LENGTH: Duration = Duration::from_millis(350);

pub fn toggle_card<M>(name: &'static str, active: bool) -> ToggleCard<M> {
    ToggleCard {
        name,
        active,
        ..ToggleCard::default()
    }
}

pub struct ToggleCard<M> {
    icon: Option<Icon>,
    name: &'static str,
    height: Length,
    width: Length,
    active: bool,
    on_press: Option<M>,
    on_long_press: Option<M>,
}

impl<M> Default for ToggleCard<M> {
    fn default() -> Self {
        Self {
            icon: None,
            name: "",
            height: Length::Shrink,
            width: Length::Fill,
            active: false,
            on_press: None,
            on_long_press: None,
        }
    }
}

impl<M> ToggleCard<M> {
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
        let style = match (self.active, state.mouse_down_start) {
            (true, None) => Style::Active,
            (true, Some(_)) => Style::ActiveHover,
            (false, None) => Style::Inactive,
            (false, Some(_)) => Style::InactiveHover,
        };

        let icon = self.icon.map(|icon| {
            svg(icon)
                .height(32)
                .width(32)
                .style(Svg::Custom(Box::new(style)))
        });

        let name = text(self.name).size(14).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let col = if let Some(icon) = icon {
            column![icon, name]
        } else {
            column![name]
        };

        mouse_area(
            container(
                col.spacing(5)
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
    Active,
    ActiveHover,
    Inactive,
    InactiveHover,
}

impl container::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        match self {
            Style::Inactive => container::Appearance {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(SLATE_200)),
                border_radius: 5.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Style::InactiveHover => container::Appearance {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(SLATE_300)),
                border_radius: 5.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Style::Active => container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(Background::Color(SKY_400)),
                border_radius: 5.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Style::ActiveHover => container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(Background::Color(SKY_500)),
                border_radius: 5.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
        }
    }
}

impl svg::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> svg::Appearance {
        match self {
            Style::Active | Style::ActiveHover => svg::Appearance {
                color: Some(AMBER_200),
            },
            Style::Inactive | Style::InactiveHover => svg::Appearance {
                color: Some(Color::BLACK),
            },
        }
    }
}
