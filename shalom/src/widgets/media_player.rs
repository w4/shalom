use std::{fmt::Display, time::Duration};

use iced::{
    advanced::graphics::core::Element,
    theme::{Svg, Text},
    widget::{column as icolumn, component, container, row, slider, svg, text, Component},
    Alignment, Length, Renderer, Theme,
};

use crate::{
    theme::{
        colours::{SKY_500, SLATE_400, SLATE_600},
        Icon,
    },
    widgets::mouse_area::mouse_area,
};

pub fn media_player<M>() -> MediaPlayer<M> {
    MediaPlayer::default()
}

pub struct MediaPlayer<M> {
    height: Length,
    width: Length,
    now_playing: NowPlaying,
    on_something: Option<M>,
    track_length: Duration,
}

impl<M> Default for MediaPlayer<M> {
    fn default() -> Self {
        Self {
            height: Length::Shrink,
            width: Length::Fill,
            now_playing: NowPlaying {
                album_art: "https://i.scdn.co/image/ab67616d00004851d771166c366eff01950de570"
                    .to_string(),
                song: "Almost Had to Start a Fight/In and Out of Patience".to_string(),
                artist: "Parquet Court".to_string(),
                loved: true,
            },
            on_something: None,
            track_length: Duration::from_secs(194),
        }
    }
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
                    self.now_playing.artist.clone(),
                    self.now_playing.song.clone(),
                    false,
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
                            0.0..=self.track_length.as_secs_f64(),
                            state.track_position.as_secs_f64(),
                            Event::PositionChange
                        ),
                        text(format_time(self.track_length))
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
        .into()
    }
}

pub struct NowPlaying {
    album_art: String,
    song: String,
    artist: String,
    loved: bool,
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
