use std::{convert::identity, sync::Arc, time::Duration};

use iced::{
    futures::StreamExt,
    subscription,
    widget::{container, image::Handle, Column, Text},
    Element, Renderer, Subscription,
};
use url::Url;

use crate::{
    hass_client::MediaPlayerRepeat,
    magic::header_search::header_search,
    oracle::{MediaPlayerSpeaker, MediaPlayerSpeakerState, Oracle, Room},
    subscriptions::{download_image, find_fanart_urls, find_musicbrainz_artist, MaybePendingImage},
    theme::{darken_image, trim_transparent_padding},
    widgets,
};

#[derive(Debug)]
pub struct Listen {
    room: Room,
    oracle: Arc<Oracle>,
    speaker: Option<(&'static str, MediaPlayerSpeaker)>,
    album_art_image: Option<Handle>,
    musicbrainz_artist_id: Option<String>,
    pub background: Option<MaybePendingImage>,
    artist_logo: Option<MaybePendingImage>,
    search_query: String,
    search_open: bool,
}

impl Listen {
    pub fn new(oracle: Arc<Oracle>, room: &Room) -> Self {
        let speaker = room.speaker(&oracle);

        Self {
            room: room.clone(),
            speaker,
            oracle,
            album_art_image: None,
            musicbrainz_artist_id: None,
            background: None,
            artist_logo: None,
            search_query: String::new(),
            search_open: false,
        }
    }

    pub fn header_magic<'a>(&self, text: Text<'a>) -> Element<'a, Message> {
        header_search(
            Message::OnSearchTerm,
            Message::OnSearchVisibleChange,
            self.search_open,
            &self.search_query,
            text,
        )
        .into()
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::AlbumArtImageLoaded(handle) => {
                self.album_art_image = Some(handle);
                None
            }
            Message::FanArtLoaded(logo, background) => {
                self.background = background.map(MaybePendingImage::Loading);
                self.artist_logo = logo.map(MaybePendingImage::Loading);
                None
            }
            Message::MusicbrainzArtistLoaded(v) => {
                eprintln!("musicbrainz artist {v}");
                self.musicbrainz_artist_id = Some(v);
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
                    self.album_art_image = None;
                    self.artist_logo = None;
                    self.background = None;
                }

                if self
                    .speaker
                    .as_ref()
                    .and_then(|(_, v)| v.media_artist.as_ref())
                    != new
                        .as_ref()
                        .as_ref()
                        .and_then(|(_, v)| v.media_artist.as_ref())
                {
                    self.musicbrainz_artist_id = None;
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
            Message::BackgroundDownloaded(handle) => {
                self.background = Some(MaybePendingImage::Downloaded(handle));
                None
            }
            Message::ArtistLogoDownloaded(handle) => {
                self.artist_logo = Some(MaybePendingImage::Downloaded(handle));
                None
            }
            Message::OnSearchTerm(v) => {
                self.search_query = v;
                None
            }
            Message::OnSearchVisibleChange(v) => {
                self.search_open = v;
                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        if let Some((_, speaker)) = self.speaker.clone() {
            container(
                widgets::media_player::media_player(speaker, self.album_art_image.clone())
                    .with_artist_logo(
                        self.artist_logo
                            .as_ref()
                            .and_then(MaybePendingImage::handle),
                    )
                    .on_volume_change(Message::OnSpeakerVolumeChange)
                    .on_mute_change(Message::OnSpeakerMuteChange)
                    .on_repeat_change(Message::OnSpeakerRepeatChange)
                    .on_state_change(Message::OnSpeakerStateChange)
                    .on_position_change(Message::OnSpeakerPositionChange)
                    .on_next_track(Message::OnSpeakerNextTrack)
                    .on_previous_track(Message::OnSpeakerPreviousTrack)
                    .on_shuffle_change(Message::OnSpeakerShuffleChange),
            )
            .into()
        } else {
            Column::new().into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let album_art_subscription = if let (Some(uri), None) = (
            self.speaker
                .as_ref()
                .and_then(|(_, v)| v.entity_picture.as_ref()),
            &self.album_art_image,
        ) {
            download_image(uri.clone(), identity, Message::AlbumArtImageLoaded)
        } else {
            Subscription::none()
        };

        let musicbrainz_artist_id_subscription = if let (Some(artist), None) = (
            self.speaker
                .as_ref()
                .and_then(|(_, v)| v.media_artist.as_ref()),
            &self.musicbrainz_artist_id,
        ) {
            find_musicbrainz_artist(artist.to_string(), Message::MusicbrainzArtistLoaded)
        } else {
            Subscription::none()
        };

        let fanart_subscription = if let (None, None, Some(musicbrainz_id)) = (
            &self.background,
            &self.artist_logo,
            &self.musicbrainz_artist_id,
        ) {
            find_fanart_urls(musicbrainz_id.clone(), Message::FanArtLoaded)
        } else {
            Subscription::none()
        };

        let background_subscription =
            if let Some(MaybePendingImage::Loading(url)) = &self.background {
                download_image(
                    url.clone(),
                    |image| crate::theme::blur(&darken_image(image, 0.3), 15),
                    Message::BackgroundDownloaded,
                )
            } else {
                Subscription::none()
            };

        let logo_subscription = if let Some(MaybePendingImage::Loading(url)) = &self.artist_logo {
            download_image(
                url.clone(),
                trim_transparent_padding,
                Message::ArtistLogoDownloaded,
            )
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

        Subscription::batch([
            album_art_subscription,
            speaker_subscription,
            musicbrainz_artist_id_subscription,
            background_subscription,
            logo_subscription,
            fanart_subscription,
        ])
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
    AlbumArtImageLoaded(Handle),
    BackgroundDownloaded(Handle),
    ArtistLogoDownloaded(Handle),
    MusicbrainzArtistLoaded(String),
    FanArtLoaded(Option<Url>, Option<Url>),
    UpdateSpeaker,
    OnSpeakerVolumeChange(f32),
    OnSpeakerPositionChange(Duration),
    OnSpeakerStateChange(bool),
    OnSpeakerMuteChange(bool),
    OnSpeakerShuffleChange(bool),
    OnSpeakerRepeatChange(MediaPlayerRepeat),
    OnSpeakerNextTrack,
    OnSpeakerPreviousTrack,
    OnSearchTerm(String),
    OnSearchVisibleChange(bool),
}
