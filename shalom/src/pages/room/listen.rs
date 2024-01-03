use std::{sync::Arc, time::Duration};

use iced::{
    futures::StreamExt,
    subscription,
    widget::{container, image::Handle, Column},
    Element, Renderer, Subscription,
};
use url::Url;

use crate::{
    hass_client::MediaPlayerRepeat,
    oracle::{MediaPlayerSpeaker, MediaPlayerSpeakerState, Oracle, Room},
    subscriptions::download_image,
    widgets,
};

#[derive(Debug)]
pub struct Listen {
    room: Room,
    oracle: Arc<Oracle>,
    speaker: Option<(&'static str, MediaPlayerSpeaker)>,
    now_playing_image: Option<Handle>,
}

impl Listen {
    pub fn new(oracle: Arc<Oracle>, room: &Room) -> Self {
        let speaker = room.speaker(&oracle);

        Self {
            room: room.clone(),
            speaker,
            oracle,
            now_playing_image: None,
        }
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
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
        let mut col = Column::new();

        if let Some((_, speaker)) = self.speaker.clone() {
            col = col.push(container(
                widgets::media_player::media_player(speaker, self.now_playing_image.clone())
                    .on_volume_change(Message::OnSpeakerVolumeChange)
                    .on_mute_change(Message::OnSpeakerMuteChange)
                    .on_repeat_change(Message::OnSpeakerRepeatChange)
                    .on_state_change(Message::OnSpeakerStateChange)
                    .on_position_change(Message::OnSpeakerPositionChange)
                    .on_next_track(Message::OnSpeakerNextTrack)
                    .on_previous_track(Message::OnSpeakerPreviousTrack)
                    .on_shuffle_change(Message::OnSpeakerShuffleChange),
            ));
        }

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

        Subscription::batch([image_subscription, speaker_subscription])
    }
}

#[derive(Copy, Clone)]
pub enum Event {
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
    UpdateSpeaker,
    OnSpeakerVolumeChange(f32),
    OnSpeakerPositionChange(Duration),
    OnSpeakerStateChange(bool),
    OnSpeakerMuteChange(bool),
    OnSpeakerShuffleChange(bool),
    OnSpeakerRepeatChange(MediaPlayerRepeat),
    OnSpeakerNextTrack,
    OnSpeakerPreviousTrack,
}
