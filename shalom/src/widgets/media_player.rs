use std::{
    fmt::Display,
    time::{Duration, Instant},
};

use iced::{
    advanced::graphics::core::Element,
    theme::{Container, Slider, Svg, Text},
    widget::{
        column as icolumn, component, container, image::Handle, row, slider, svg, text, Component,
    },
    Alignment, Background, Color, Length, Renderer, Theme,
};

use crate::{
    hass_client::MediaPlayerRepeat,
    oracle::{MediaPlayerSpeaker, MediaPlayerSpeakerState},
    theme::{
        colours::{SKY_500, SLATE_400},
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
        on_volume_change: None,
        on_position_change: None,
        on_state_change: None,
        on_mute_change: None,
        on_repeat_change: None,
        on_next_track: None,
        on_previous_track: None,
        on_shuffle_change: None,
    }
}

#[derive(Clone)]
pub struct MediaPlayer<M> {
    height: Length,
    width: Length,
    device: MediaPlayerSpeaker,
    image: Option<Handle>,
    on_volume_change: Option<fn(f32) -> M>,
    on_position_change: Option<fn(Duration) -> M>,
    on_state_change: Option<fn(bool) -> M>,
    on_mute_change: Option<fn(bool) -> M>,
    on_repeat_change: Option<fn(MediaPlayerRepeat) -> M>,
    on_next_track: Option<M>,
    on_previous_track: Option<M>,
    on_shuffle_change: Option<fn(bool) -> M>,
}

impl<M> MediaPlayer<M> {
    pub fn on_volume_change(mut self, f: fn(f32) -> M) -> Self {
        self.on_volume_change = Some(f);
        self
    }

    pub fn on_position_change(mut self, f: fn(Duration) -> M) -> Self {
        self.on_position_change = Some(f);
        self
    }

    pub fn on_state_change(mut self, f: fn(bool) -> M) -> Self {
        self.on_state_change = Some(f);
        self
    }

    pub fn on_mute_change(mut self, f: fn(bool) -> M) -> Self {
        self.on_mute_change = Some(f);
        self
    }

    pub fn on_repeat_change(mut self, f: fn(MediaPlayerRepeat) -> M) -> Self {
        self.on_repeat_change = Some(f);
        self
    }

    pub fn on_next_track(mut self, msg: M) -> Self {
        self.on_next_track = Some(msg);
        self
    }

    pub fn on_previous_track(mut self, msg: M) -> Self {
        self.on_previous_track = Some(msg);
        self
    }

    pub fn on_shuffle_change(mut self, f: fn(bool) -> M) -> Self {
        self.on_shuffle_change = Some(f);
        self
    }
}

impl<M: Clone> Component<M, Renderer> for MediaPlayer<M> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::VolumeChange(new) => {
                state.overridden_volume = Some(new);
                None
            }
            Event::PositionChange(new) => {
                state.overridden_position = Some(Duration::from_secs_f64(new));
                None
            }
            Event::TogglePlaying => self
                .on_state_change
                .map(|f| f(!self.device.state.is_playing())),
            Event::ToggleMute => self.on_mute_change.map(|f| f(!self.device.muted)),
            Event::ToggleRepeat => self.on_repeat_change.map(|f| f(self.device.repeat.next())),
            Event::OnVolumeRelease => self
                .on_volume_change
                .zip(state.overridden_volume.take())
                .map(|(f, vol)| f(vol)),
            Event::OnPositionRelease => self
                .on_position_change
                .zip(state.overridden_position.take())
                .map(|(f, pos)| f(pos)),
            Event::PreviousTrack => {
                let last_press = state
                    .last_previous_click
                    .as_ref()
                    .map_or(Duration::MAX, Instant::elapsed);
                state.last_previous_click = Some(Instant::now());

                if last_press > Duration::from_secs(2) {
                    self.on_position_change.map(|f| f(Duration::ZERO))
                } else {
                    self.on_previous_track.clone()
                }
            }
            Event::NextTrack => self.on_next_track.clone(),
            Event::ToggleShuffle => self.on_shuffle_change.map(|f| f(!self.device.shuffle)),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let icon_style = |v| Svg::Custom(Box::new(if v { Style::Active } else { Style::Inactive }));

        let position = state
            .overridden_position
            .or(self.device.actual_media_position)
            .unwrap_or_default();

        let volume = state.overridden_volume.unwrap_or(self.device.volume);

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
                        mouse_area(
                            svg(Icon::Shuffle)
                                .height(24)
                                .width(24)
                                .style(icon_style(self.device.shuffle)),
                        )
                        .on_press(Event::ToggleShuffle),
                        mouse_area(
                            svg(Icon::Backward)
                                .height(28)
                                .width(28)
                                .style(icon_style(false))
                        )
                        .on_press(Event::PreviousTrack),
                        mouse_area(
                            svg(if self.device.state == MediaPlayerSpeakerState::Playing {
                                Icon::Pause
                            } else {
                                Icon::Play
                            })
                            .height(42)
                            .width(42)
                            .style(icon_style(false))
                        )
                        .on_press(Event::TogglePlaying),
                        mouse_area(
                            svg(Icon::Forward)
                                .height(28)
                                .width(28)
                                .style(icon_style(false))
                        )
                        .on_press(Event::NextTrack),
                        mouse_area(
                            svg(match self.device.repeat {
                                MediaPlayerRepeat::Off | MediaPlayerRepeat::All => Icon::Repeat,
                                MediaPlayerRepeat::One => Icon::Repeat1,
                            })
                            .height(28)
                            .width(28)
                            .style(icon_style(self.device.repeat != MediaPlayerRepeat::Off)),
                        )
                        .on_press(Event::ToggleRepeat),
                    ]
                    .spacing(14)
                    .align_items(Alignment::Center),
                    row![
                        text(format_time(position))
                            .style(Text::Color(SLATE_400))
                            .size(12)
                            .width(Length::FillPortion(10)),
                        slider(
                            0.0..=self.device.media_duration.unwrap_or_default().as_secs_f64(),
                            position.as_secs_f64(),
                            Event::PositionChange
                        )
                        .on_release(Event::OnPositionRelease)
                        .style(Slider::Custom(Box::new(SliderStyle)))
                        .width(Length::FillPortion(80)),
                        text(format_time(self.device.media_duration.unwrap_or_default()))
                            .style(Text::Color(SLATE_400))
                            .size(12)
                            .width(Length::FillPortion(10)),
                    ]
                    .spacing(14)
                    .align_items(Alignment::Center),
                ]
                .spacing(8)
                .align_items(Alignment::Center)
                .width(Length::FillPortion(12)),
                row![
                    mouse_area(
                        svg(if self.device.muted {
                            Icon::SpeakerMuted
                        } else {
                            Icon::Speaker
                        })
                        .height(16)
                        .width(16)
                        .style(icon_style(false)),
                    )
                    .on_press(Event::ToggleMute),
                    slider(0.0..=1.0, volume, Event::VolumeChange)
                        .width(128)
                        .step(0.01)
                        .on_release(Event::OnVolumeRelease)
                        .style(Slider::Custom(Box::new(SliderStyle))),
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
        .style(Container::Custom(Box::new(Style::Inactive)))
        .padding(20)
        .into()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct State {
    overridden_position: Option<Duration>,
    overridden_volume: Option<f32>,
    last_previous_click: Option<Instant>,
}

#[derive(Clone)]
pub enum Event {
    TogglePlaying,
    ToggleMute,
    ToggleRepeat,
    ToggleShuffle,
    VolumeChange(f32),
    PositionChange(f64),
    OnVolumeRelease,
    OnPositionRelease,
    PreviousTrack,
    NextTrack,
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

struct SliderStyle;

impl slider::StyleSheet for SliderStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> slider::Appearance {
        let palette = style.extended_palette();

        let handle = slider::Handle {
            shape: slider::HandleShape::Rectangle {
                width: 0,
                border_radius: 0.0.into(),
            },
            color: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        };

        slider::Appearance {
            rail: slider::Rail {
                colors: (palette.primary.base.color, palette.secondary.base.color),
                width: 4.0,
                border_radius: 4.0.into(),
            },
            handle,
        }
    }

    fn hovered(&self, style: &Self::Style) -> slider::Appearance {
        let palette = style.extended_palette();

        let handle = slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            color: palette.background.base.color,
            border_color: palette.primary.base.color,
            border_width: 1.0,
        };

        slider::Appearance {
            rail: slider::Rail {
                colors: (palette.primary.base.color, palette.secondary.base.color),
                width: 4.0,
                border_radius: 4.0.into(),
            },
            handle,
        }
    }

    fn dragging(&self, style: &Self::Style) -> slider::Appearance {
        self.hovered(style)
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
        container::Appearance {
            text_color: None,
            background: Some(Background::Color(Color {
                a: 0.8,
                ..Color::BLACK
            })),
            border_radius: 10.0.into(),
            border_width: 0.,
            border_color: Color::default(),
        }
    }
}

impl svg::StyleSheet for Style {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> svg::Appearance {
        let color = match self {
            Self::Active => SKY_500,
            Self::Inactive => Color::WHITE,
        };

        svg::Appearance { color: Some(color) }
    }
}
