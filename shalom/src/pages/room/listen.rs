mod search;

use std::{borrow::Cow, convert::identity, iter, sync::Arc, time::Duration};

use iced::{
    futures::{future, future::Either, stream, stream::FuturesUnordered, FutureExt, StreamExt},
    subscription,
    widget::{container, image::Handle, lazy, Column, Text},
    Element, Length, Renderer, Subscription, Theme,
};
use itertools::Itertools;
use serde::Deserialize;
use url::Url;
use yoke::{Yoke, Yokeable};

use crate::{
    config::Config,
    hass_client::MediaPlayerRepeat,
    magic::header_search::header_search,
    oracle::{MediaPlayerSpeaker, MediaPlayerSpeakerState, Oracle, Room},
    pages::room::listen::search::SearchResult,
    subscriptions::{
        download_image, find_fanart_urls, find_musicbrainz_artist, load_image, MaybePendingImage,
    },
    theme::{darken_image, trim_transparent_padding, Image},
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
    search: SearchState,
    config: Arc<Config>,
}

impl Listen {
    pub fn new(oracle: Arc<Oracle>, room: &Room, config: Arc<Config>) -> Self {
        let speaker = room.speaker(&oracle);

        Self {
            room: room.clone(),
            speaker,
            oracle,
            album_art_image: None,
            musicbrainz_artist_id: None,
            background: None,
            artist_logo: None,
            search: SearchState::Closed,
            config,
        }
    }

    pub fn header_magic(&self, text: Text<'static>) -> Element<'static, Message> {
        lazy(self.search.clone(), move |search| {
            let (open, query) = if let Some(v) = search.search() {
                (true, v)
            } else {
                (false, "")
            };

            header_search(
                Message::OnSearchTerm,
                Message::OnSearchVisibleChange,
                open,
                query,
                text.clone(),
            )
        })
        .into()
    }

    #[allow(clippy::too_many_lines)]
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
                self.search = self.search.open(v);
                None
            }
            Message::OnSearchVisibleChange(v) => {
                self.search = if v {
                    SearchState::Open {
                        search: String::new(),
                        results_search: String::new(),
                        results: Ok(vec![]),
                    }
                } else {
                    SearchState::Closed
                };
                None
            }
            Message::SpotifySearchResult((res, search)) => {
                if self.search.search() != Some(&search) {
                    return None;
                }

                if let SearchState::Open { results, .. } = &mut self.search {
                    if let Ok(results) = results {
                        results.push(res);
                    } else {
                        *results = Ok(vec![res]);
                    }
                }

                None
            }
            Message::SpotifySearchResultError((res, search)) => {
                if self.search.search() != Some(&search) {
                    return None;
                }

                if let SearchState::Open { results, .. } = &mut self.search {
                    *results = Err(res);
                }

                None
            }
            Message::OnPlayTrack(uri) => Some(Event::PlayTrack(self.speaker.as_ref()?.0, uri)),
        }
    }

    pub fn view(&self, style: &Theme) -> Element<'_, Message, Renderer> {
        if self.search.is_open() {
            container(
                search::search(style.clone(), self.search.results())
                    .on_track_press(Message::OnPlayTrack),
            )
            .padding([0, 40, 40, 40])
            .width(Length::Fill)
            .into()
        } else if let Some((_, speaker)) = self.speaker.clone() {
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

        let spotify_result = if let SearchState::Open { search, .. } = &self.search {
            search_spotify(search, &self.config.spotify.token)
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
            spotify_result,
        ])
    }
}

#[derive(Debug, Hash, Clone)]
pub enum SearchState {
    Open {
        search: String,
        results_search: String,
        results: Result<Vec<SearchResult>, String>,
    },
    Closed,
}

impl SearchState {
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open { search, .. } if !search.is_empty())
    }

    pub fn results(&self) -> search::SearchState<'_> {
        match self {
            Self::Open {
                results,
                results_search,
                ..
            } => match results {
                Ok(v) if results_search.is_empty() && v.is_empty() => search::SearchState::NotReady,
                Ok(v) => search::SearchState::Ready(v.as_slice()),
                Err(e) => search::SearchState::Error(e),
            },
            Self::Closed => search::SearchState::NotReady,
        }
    }

    pub fn search(&self) -> Option<&str> {
        match self {
            Self::Open { search, .. } => Some(search),
            Self::Closed => None,
        }
    }

    pub fn open(&self, search: String) -> Self {
        match self {
            Self::Open { results_search, .. } => Self::Open {
                search,
                results_search: results_search.clone(),
                results: Ok(vec![]),
            },
            Self::Closed => Self::Open {
                search,
                results_search: String::new(),
                results: Ok(vec![]),
            },
        }
    }
}

#[derive(Clone)]
pub enum Event {
    SetSpeakerVolume(&'static str, f32),
    SetSpeakerPosition(&'static str, Duration),
    SetSpeakerPlaying(&'static str, bool),
    SetSpeakerMuted(&'static str, bool),
    SetSpeakerShuffle(&'static str, bool),
    SetSpeakerRepeat(&'static str, MediaPlayerRepeat),
    SpeakerNextTrack(&'static str),
    SpeakerPreviousTrack(&'static str),
    PlayTrack(&'static str, String),
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
    SpotifySearchResult((SearchResult, String)),
    SpotifySearchResultError((String, String)),
    OnPlayTrack(String),
}

fn search_spotify(search_param: &str, token: &str) -> Subscription<Message> {
    if search_param.is_empty() {
        return Subscription::none();
    }

    let token = token.to_string();

    let search = search_param.to_string();
    subscription::run_with_id(
        format!("search-{search}"),
        stream::once(async move {
            eprintln!("sending search {search}");

            let mut url = Url::parse("https://api.spotify.com/v1/search").unwrap();
            url.query_pairs_mut()
                .append_pair("q", &search)
                .append_pair("type", "album,artist,playlist,track")
                .append_pair("market", "GB")
                .append_pair("limit", "20");

            let res = reqwest::Client::new()
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();

            eprintln!("{search} - {}", std::str::from_utf8(res.as_ref()).unwrap());

            (
                Yoke::attach_to_cart(res, |s| serde_json::from_str(s).unwrap()),
                search,
            )
        })
        .flat_map(
            |(res, search): (Yoke<SpotifySearchResult<'static>, String>, String)| {
                let res = res.get();

                if let Some(error) = &res.error {
                    return Either::Left(stream::iter(iter::once(
                        Message::SpotifySearchResultError((error.message.to_string(), search)),
                    )));
                }

                let results = FuturesUnordered::new();

                for track in &res.tracks.items {
                    let image_url = track.album.images.last().map(|v| v.url.to_string());
                    let track_name = track.name.to_string();
                    let artist_name = track.artists.iter().map(|v| &v.name).join(", ");
                    let uri = track.uri.to_string();

                    results.push(tokio::spawn(
                        async move {
                            let image = load_album_art(image_url).await;
                            SearchResult::track(image, track_name, artist_name, uri)
                        }
                        .boxed(),
                    ));
                }

                for artist in &res.artists.items {
                    let image_url = artist.images.last().map(|v| v.url.to_string());
                    let artist_name = artist.name.to_string();
                    let uri = artist.uri.to_string();

                    results.push(tokio::spawn(
                        async move {
                            let image = load_album_art(image_url).await;
                            SearchResult::artist(image, artist_name, uri)
                        }
                        .boxed(),
                    ));
                }

                for albums in &res.albums.items {
                    let image_url = albums.images.last().map(|v| v.url.to_string());
                    let album_name = albums.name.to_string();
                    let uri = albums.uri.to_string();

                    results.push(tokio::spawn(
                        async move {
                            let image = load_album_art(image_url).await;
                            SearchResult::album(image, album_name, uri)
                        }
                        .boxed(),
                    ));
                }

                for playlist in &res.playlists.items {
                    let image_url = playlist.images.last().map(|v| v.url.to_string());
                    let playlist_name = playlist.name.to_string();
                    let uri = playlist.uri.to_string();

                    results.push(tokio::spawn(
                        async move {
                            let image = load_album_art(image_url).await;
                            SearchResult::playlist(image, playlist_name, uri)
                        }
                        .boxed(),
                    ));
                }

                Either::Right(
                    results
                        .filter_map(|v| future::ready(v.ok()))
                        .zip(stream::repeat(search))
                        .map(Message::SpotifySearchResult),
                )
            },
        ),
    )
}

async fn load_album_art(image_url: Option<String>) -> Handle {
    if let Some(image_url) = image_url {
        load_image(image_url, identity).await
    } else {
        Image::UnknownArtist.into()
    }
}

#[derive(Deserialize, Yokeable)]
pub struct SpotifySearchResult<'a> {
    #[serde(borrow, default)]
    tracks: SpotifySearchResultWrapper<SpotifyTrack<'a>>,
    #[serde(borrow, default)]
    artists: SpotifySearchResultWrapper<SpotifyArtist<'a>>,
    #[serde(borrow, default)]
    albums: SpotifySearchResultWrapper<SpotifyAlbum<'a>>,
    #[serde(borrow, default)]
    playlists: SpotifySearchResultWrapper<SpotifyPlaylist<'a>>,
    #[serde(borrow, default)]
    error: Option<SpotifyError<'a>>,
}

#[derive(Deserialize)]
pub struct SpotifyError<'a> {
    message: &'a str,
}

#[derive(Deserialize)]
pub struct SpotifySearchResultWrapper<T> {
    items: Vec<T>,
}

impl<T> Default for SpotifySearchResultWrapper<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Yokeable)]
pub struct SpotifyTrack<'a> {
    #[serde(borrow)]
    album: SpotifyAlbum<'a>,
    #[serde(borrow)]
    artists: Vec<SpotifyArtist<'a>>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    uri: Cow<'a, str>,
}

#[derive(Deserialize, Yokeable)]
#[allow(dead_code)]
pub struct SpotifyAlbum<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow, default)]
    images: Vec<SpotifyImage<'a>>,
    #[serde(borrow, default)]
    artists: Vec<SpotifyArtist<'a>>,
    #[serde(borrow)]
    uri: Cow<'a, str>,
}

#[derive(Deserialize, Yokeable)]
pub struct SpotifyPlaylist<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow, default)]
    images: Vec<SpotifyImage<'a>>,
    #[serde(borrow)]
    uri: Cow<'a, str>,
}

#[derive(Deserialize)]
pub struct SpotifyArtist<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow, default)]
    images: Vec<SpotifyImage<'a>>,
    #[serde(borrow)]
    uri: Cow<'a, str>,
}

#[derive(Deserialize)]
pub struct SpotifyImage<'a> {
    #[serde(borrow)]
    url: Cow<'a, str>,
}
