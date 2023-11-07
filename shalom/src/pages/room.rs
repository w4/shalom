use std::{collections::BTreeMap, sync::Arc, time::Duration};

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    futures::StreamExt,
    subscription,
    widget::{container, image::Handle, text, Column, Row},
    Font, Renderer, Subscription,
};
use url::Url;

use crate::{
    hass_client::MediaPlayerRepeat,
    oracle::{Light, MediaPlayerSpeaker, MediaPlayerSpeakerState, Oracle},
    subscriptions::download_image,
    theme::Icon,
    widgets,
    widgets::colour_picker::colour_from_hsb,
};

#[derive(Debug)]
pub struct Room {
    id: &'static str,
    oracle: Arc<Oracle>,
    room: crate::oracle::Room,
    speaker: Option<(&'static str, MediaPlayerSpeaker)>,
    now_playing_image: Option<Handle>,
    lights: BTreeMap<&'static str, Light>,
}

impl Room {
    pub fn new(id: &'static str, oracle: Arc<Oracle>) -> Self {
        let room = oracle.room(id).clone();
        let speaker = room.speaker(&oracle);

        let lights = room.lights(&oracle);

        Self {
            id,
            oracle,
            room,
            speaker,
            now_playing_image: None,
            lights,
        }
    }

    pub fn room_id(&self) -> &'static str {
        self.id
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::SetLightState(id, state) => {
                // give instant feedback before we get the event back from hass
                if let Some(light) = self.lights.get_mut(id) {
                    light.on = Some(state);
                }

                Some(Event::SetLightState(id, state))
            }
            Message::OpenLightOptions(id) => Some(Event::OpenLightContextMenu(id)),
            Message::NowPlayingImageLoaded(url, handle) => {
                if self
                    .speaker
                    .as_ref()
                    .and_then(|(_, v)| v.entity_picture.as_ref())
                    == Some(&url)
                {
                    self.now_playing_image = Some(handle);
                }

                None
            }
            Message::UpdateSpeaker => {
                let new = self.room.speaker(&self.oracle);

                if self
                    .speaker
                    .as_ref()
                    .and_then(|(_, v)| v.entity_picture.as_ref())
                    != new
                        .as_ref()
                        .as_ref()
                        .and_then(|(_, v)| v.entity_picture.as_ref())
                {
                    self.now_playing_image = None;
                }

                self.speaker = new;

                None
            }
            Message::UpdateLight(entity_id) => {
                if let Some(light) = self.oracle.fetch_light(entity_id) {
                    self.lights.insert(entity_id, light);
                }

                None
            }
            Message::OnSpeakerVolumeChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.volume = new;
                Some(Event::SetSpeakerVolume(id, new))
            }
            Message::OnSpeakerPositionChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.actual_media_position = Some(new);
                Some(Event::SetSpeakerPosition(id, new))
            }
            Message::OnSpeakerStateChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.state = if new {
                    MediaPlayerSpeakerState::Playing
                } else {
                    MediaPlayerSpeakerState::Paused
                };
                Some(Event::SetSpeakerPlaying(id, new))
            }
            Message::OnSpeakerMuteChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.muted = new;
                Some(Event::SetSpeakerMuted(id, new))
            }
            Message::OnSpeakerRepeatChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.repeat = new;
                Some(Event::SetSpeakerRepeat(id, new))
            }
            Message::OnSpeakerNextTrack => Some(Event::SpeakerNextTrack(self.speaker.as_ref()?.0)),
            Message::OnSpeakerPreviousTrack => {
                Some(Event::SpeakerPreviousTrack(self.speaker.as_ref()?.0))
            }
            Message::OnSpeakerShuffleChange(new) => {
                let (id, speaker) = self.speaker.as_mut()?;
                speaker.shuffle = new;
                Some(Event::SetSpeakerShuffle(id, new))
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let header = text(self.room.name.as_ref()).size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let light = |id, light: &Light| {
            let mut toggle_card = widgets::toggle_card::toggle_card(
                &light.friendly_name,
                light.on.unwrap_or_default(),
                light.on.is_none(),
            )
            .icon(Icon::Bulb)
            .active_icon_colour(
                light
                    .hs_color
                    .zip(light.brightness)
                    .map(|((h, s), b)| colour_from_hsb(h, s, b / 255.)),
            );

            if let Some(state) = light.on {
                toggle_card = toggle_card
                    .on_press(Message::SetLightState(id, !state))
                    .on_long_press(Message::OpenLightOptions(id));
            }

            toggle_card
        };

        let mut col = Column::new().spacing(20).padding(40).push(header);

        if let Some((_, speaker)) = self.speaker.clone() {
            col = col.push(
                container(
                    widgets::media_player::media_player(speaker, self.now_playing_image.clone())
                        .on_volume_change(Message::OnSpeakerVolumeChange)
                        .on_mute_change(Message::OnSpeakerMuteChange)
                        .on_repeat_change(Message::OnSpeakerRepeatChange)
                        .on_state_change(Message::OnSpeakerStateChange)
                        .on_position_change(Message::OnSpeakerPositionChange)
                        .on_next_track(Message::OnSpeakerNextTrack)
                        .on_previous_track(Message::OnSpeakerPreviousTrack)
                        .on_shuffle_change(Message::OnSpeakerShuffleChange),
                )
                .padding([12, 0, 24, 0]),
            );
        }

        let lights = Row::with_children(
            self.lights
                .iter()
                .map(|(id, item)| light(*id, item))
                .map(Element::from)
                .collect::<Vec<_>>(),
        )
        .spacing(10);
        col = col.push(lights);

        col.into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let image_subscription = if let (Some(uri), None) = (
            self.speaker
                .as_ref()
                .and_then(|(_, v)| v.entity_picture.as_ref()),
            &self.now_playing_image,
        ) {
            download_image("now-playing", uri.clone(), |_, url, handle| {
                Message::NowPlayingImageLoaded(url, handle)
            })
        } else {
            Subscription::none()
        };

        let speaker_subscription = if let Some(speaker_id) = self.speaker.as_ref().map(|(k, _)| *k)
        {
            subscription::run_with_id(
                speaker_id,
                self.oracle
                    .subscribe_id(speaker_id)
                    .map(|()| Message::UpdateSpeaker),
            )
        } else {
            Subscription::none()
        };

        let light_subscriptions = Subscription::batch(self.lights.keys().copied().map(|key| {
            subscription::run_with_id(
                key,
                self.oracle
                    .subscribe_id(key)
                    .map(|()| Message::UpdateLight(key)),
            )
        }));

        Subscription::batch([
            image_subscription,
            speaker_subscription,
            light_subscriptions,
        ])
    }
}

pub enum Event {
    OpenLightContextMenu(&'static str),
    SetLightState(&'static str, bool),
    SetSpeakerVolume(&'static str, f32),
    SetSpeakerPosition(&'static str, Duration),
    SetSpeakerPlaying(&'static str, bool),
    SetSpeakerMuted(&'static str, bool),
    SetSpeakerShuffle(&'static str, bool),
    SetSpeakerRepeat(&'static str, MediaPlayerRepeat),
    SpeakerNextTrack(&'static str),
    SpeakerPreviousTrack(&'static str),
}

#[derive(Clone, Debug)]
pub enum Message {
    NowPlayingImageLoaded(Url, Handle),
    SetLightState(&'static str, bool),
    OpenLightOptions(&'static str),
    UpdateSpeaker,
    UpdateLight(&'static str),
    OnSpeakerVolumeChange(f32),
    OnSpeakerPositionChange(Duration),
    OnSpeakerStateChange(bool),
    OnSpeakerMuteChange(bool),
    OnSpeakerShuffleChange(bool),
    OnSpeakerRepeatChange(MediaPlayerRepeat),
    OnSpeakerNextTrack,
    OnSpeakerPreviousTrack,
}
