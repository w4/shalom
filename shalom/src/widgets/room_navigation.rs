use iced::{
    advanced::graphics::core::Element,
    alignment::Vertical,
    font::{Stretch, Weight},
    theme,
    widget::{column, component, container, horizontal_rule, rule, svg, text, Component},
    Alignment, Background, Color, ContentFit, Font, Length, Renderer, Theme,
};

use super::mouse_area::mouse_area;
use crate::theme::{
    colours::{SKY_500, SLATE_200},
    Icon,
};

pub struct RoomNavigation<M> {
    _phantom: std::marker::PhantomData<M>,
    width: Length,
    current: Page,
    on_change: Option<fn(Page) -> M>,
    on_exit: Option<M>,
}

impl<M> RoomNavigation<M> {
    pub fn new(current: Page) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            width: Length::Fill,
            current,
            on_change: None,
            on_exit: None,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn on_change(mut self, on_change: fn(Page) -> M) -> Self {
        self.on_change = Some(on_change);
        self
    }

    pub fn on_exit(mut self, event: M) -> Self {
        self.on_exit = Some(event);
        self
    }
}

impl<M: Clone> Component<M, Renderer> for RoomNavigation<M> {
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::Change(page) => self.on_change.map(|v| v(page)),
            Event::Exit => self.on_exit.clone(),
        }
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let section = |icon: Icon, t: &'static str, state: Style, page| {
            mouse_area(
                container(
                    column![
                        svg(icon)
                            .height(Length::Fixed(64.))
                            .width(Length::Fixed(64.))
                            .style(theme::Svg::Custom(Box::new(state))),
                        text(t).size(18.).font(Font {
                            weight: Weight::Bold,
                            stretch: Stretch::Condensed,
                            ..Font::with_name("Helvetica Neue")
                        }),
                    ]
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .padding(12.),
                )
                .style(theme::Container::Custom(Box::new(state)))
                .width(Length::Fill),
            )
            .on_press(Event::Change(page))
        };

        let s = |p: &[Page]| {
            if p.contains(&self.current) {
                Style::Active
            } else {
                Style::Inactive
            }
        };

        let exit = container(
            mouse_area(
                svg(Icon::Back)
                    .height(32)
                    .width(32)
                    .content_fit(ContentFit::None),
            )
            .on_press(Event::Exit),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Vertical::Bottom)
        .padding(40);

        column![
            section(Icon::Speaker, "Listen", s(&[Page::Listen]), Page::Listen),
            horizontal_rule(1).style(theme::Rule::Custom(Box::new(s(&[
                Page::Listen,
                Page::Climate
            ])))),
            section(Icon::Hvac, "Climate", s(&[Page::Climate]), Page::Climate),
            horizontal_rule(1).style(theme::Rule::Custom(Box::new(s(&[
                Page::Climate,
                Page::Lights
            ])))),
            section(Icon::Bulb, "Lights", s(&[Page::Lights]), Page::Lights),
            exit,
        ]
        .width(self.width)
        .height(Length::Fill)
        .into()
    }
}

#[derive(Copy, Clone)]
pub enum Event {
    Change(Page),
    Exit,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Listen,
    Climate,
    Lights,
}

impl<'a, M> From<RoomNavigation<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: RoomNavigation<M>) -> Self {
        component(card)
    }
}

#[derive(Copy, Clone)]
pub enum Style {
    Active,
    Inactive,
}

impl container::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        match self {
            Self::Active => container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(Background::Color(SKY_500)),
                border_radius: 0.0.into(),
                border_width: 0.0,
                border_color: Color::default(),
            },
            Self::Inactive => container::Appearance {
                text_color: Some(Color::BLACK),
                background: Some(Background::Color(Color::WHITE)),
                border_radius: 0.0.into(),
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
            Self::Active => svg::Appearance {
                color: Some(Color::WHITE),
            },
            Self::Inactive => svg::Appearance {
                color: Some(Color::BLACK),
            },
        }
    }
}

impl rule::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> rule::Appearance {
        match self {
            Self::Active => rule::Appearance {
                color: Color::WHITE,
                width: 1,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
            },
            Self::Inactive => rule::Appearance {
                color: SLATE_200,
                width: 1,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
            },
        }
    }
}
