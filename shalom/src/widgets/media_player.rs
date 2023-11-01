use std::{fmt::Display, time::Duration};

use iced::{
    advanced::graphics::core::Element,
    theme::{Svg, Text},
    widget::{
        column as icolumn, component, container, image::Handle, row, slider, svg, text, Component,
    },
    Alignment, Length, Renderer, Theme,
};

use crate::{
    oracle::MediaPlayerSpeaker,
    theme::{
        colours::{SKY_500, SLATE_400, SLATE_600},
        Icon,
    },
    widgets::mouse_area::mouse_area,
};

pub fn media_player<M>(device: MediaPlayerSpeaker, image: Option<Handle>) -> MediaPlayer<M> {
    MediaPlayer {
        height: Length::Shrink,
        width: Length::Fill,
        device,
        image,
        _on_something: None,
    }
}

#[derive(Clone)]
pub struct MediaPlayer<M> {
    height: Length,
    width: Length,
    device: MediaPlayerSpeaker,
    image: Option<Handle>,
    _on_something: Option<M>,
}

impl<M> Component<M, Renderer> for MediaPlayer<M> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::VolumeChange(new) => {
                state.volume = new;
                None
            }
            Event::PositionChange(new) => {
                state.track_position = Duration::from_secs_f64(new);
                None
            }
            Event::TogglePlaying => {
                state.playing = !state.playing;
                None
            }
            Event::ToggleMute => {
                state.muted = !state.muted;
                None
            }
            Event::ToggleRepeat => {
                state.repeat = !state.repeat;
                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let icon_style = |v| Svg::Custom(Box::new(if v { Style::Active } else { Style::Inactive }));

        container(
            row![
                container(crate::widgets::track_card::track_card(
                    self.device
                        .media_artist
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    self.device
                        .media_title
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    self.image.clone(),
                ),)
                .width(Length::FillPortion(8)),
                icolumn![
                    // container(
                    //     svg(Icon::Hamburger)
                    //         .height(30)
                    //         .width(30),
                    // )
                    // .align_x(Horizontal::Right)
                    // .align_y(Vertical::Center)
                    // .width(Length::Fill),
                    row![
                        svg(Icon::Backward)
                            .height(24)
                            .width(24)
                            .style(icon_style(false)),
                        mouse_area(
                            svg(if state.playing {
                                Icon::Pause
                            } else {
                                Icon::Play
                            })
                            .height(24)
                            .width(24)
                            .style(icon_style(false))
                        )
                        .on_press(Event::TogglePlaying),
                        svg(Icon::Forward)
                            .height(24)
                            .width(24)
                            .style(icon_style(false)),
                        mouse_area(
                            svg(Icon::Repeat)
                                .height(24)
                                .width(24)
                                .style(icon_style(state.repeat)),
                        )
                        .on_press(Event::ToggleRepeat),
                    ]
                    .spacing(14),
                    row![
                        text(format_time(state.track_position))
                            .style(Text::Color(SLATE_400))
                            .size(12),
                        slider(
                            0.0..=self.device.media_duration.unwrap_or_default().as_secs_f64(),
                            state.track_position.as_secs_f64(),
                            Event::PositionChange
                        ),
                        text(format_time(self.device.media_duration.unwrap_or_default()))
                            .style(Text::Color(SLATE_400))
                            .size(12),
                    ]
                    .spacing(14)
                    .align_items(Alignment::Center),
                ]
                .spacing(8)
                .align_items(Alignment::Center)
                .width(Length::FillPortion(12)),
                row![
                    mouse_area(
                        svg(if state.muted {
                            Icon::SpeakerMuted
                        } else {
                            Icon::Speaker
                        })
                        .height(16)
                        .width(16)
                        .style(icon_style(false)),
                    )
                    .on_press(Event::ToggleMute),
                    slider(0..=100, state.volume, Event::VolumeChange).width(128),
                ]
                .align_items(Alignment::Center)
                .width(Length::FillPortion(4))
                .spacing(12),
            ]
            .align_items(Alignment::Center)
            .spacing(48),
        )
        .height(self.height)
        .width(self.width)
        .center_x()
        .center_y()
        // .style(Container::Custom(Box::new(Style::Inactive)))
        .into()
    }
}

#[derive(Default)]
pub struct State {
    muted: bool,
    volume: u8,
    track_position: Duration,
    playing: bool,
    repeat: bool,
}

#[derive(Clone)]
pub enum Event {
    TogglePlaying,
    ToggleMute,
    ToggleRepeat,
    VolumeChange(u8),
    PositionChange(f64),
}

impl<'a, M> From<MediaPlayer<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: MediaPlayer<M>) -> Self {
        component(card)
    }
}

fn format_time(duration: Duration) -> impl Display {
    let secs = duration.as_secs();
    let minutes = secs / 60;
    let seconds = secs % 60;

    format!("{minutes:02}:{seconds:02}")
}

#[derive(Copy, Clone)]
pub enum Style {
    Active,
    Inactive,
}

// impl container::StyleSheet for Style {
//     type Style = Theme;
//
//     fn appearance(&self, style: &Self::Style) -> container::Appearance {
//         container::Appearance {
//             text_color: None,
//             background: Some(Background::Color(SLATE_200)),
//             border_radius: Default::default(),
//             border_width: 0.0,
//             border_color: Default::default(),
//         }
//     }
// }

impl svg::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> svg::Appearance {
        let color = match self {
            Self::Active => SKY_500,
            Self::Inactive => SLATE_600,
        };

        svg::Appearance { color: Some(color) }
    }
}
