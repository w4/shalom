use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    str::FromStr,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use atomic::Atomic;
use bytemuck::NoUninit;
use iced::futures::{future, Stream, StreamExt};
use internment::Intern;
use itertools::Itertools;
use parking_lot::Mutex;
use time::OffsetDateTime;
use tokio::{
    sync::{broadcast, broadcast::error::RecvError},
    time::MissedTickBehavior,
};
use tokio_stream::wrappers::BroadcastStream;
use url::Url;
use yoke::Yoke;

use crate::{
    hass_client::{
        responses::{
            Area, AreaRegistryList, ColorMode, DeviceRegistryList, Entity, EntityRegistryList,
            StateAttributes, StateLightAttributes, StateMediaPlayerAttributes,
            StateWeatherAttributes, StatesList, WeatherCondition,
        },
        CallServiceRequestData, CallServiceRequestLight, CallServiceRequestLightTurnOn,
        CallServiceRequestMediaPlayer, CallServiceRequestMediaPlayerMediaSeek,
        CallServiceRequestMediaPlayerRepeatSet, CallServiceRequestMediaPlayerShuffleSet,
        CallServiceRequestMediaPlayerVolumeMute, CallServiceRequestMediaPlayerVolumeSet, Event,
        HassRequestKind, MediaPlayerRepeat,
    },
    widgets::colour_picker::clamp_to_u8,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<&'static str, Room>,
    weather: Atomic<Weather>,
    media_players: Mutex<BTreeMap<&'static str, MediaPlayer>>,
    lights: Mutex<BTreeMap<&'static str, Light>>,
    entity_updates: broadcast::Sender<Arc<str>>,
}

impl Oracle {
    pub async fn new(hass_client: crate::hass_client::Client) -> Arc<Self> {
        let (rooms, devices, entities, states) = tokio::join!(
            hass_client.request::<AreaRegistryList<'_>>(HassRequestKind::AreaRegistry),
            hass_client.request::<DeviceRegistryList<'_>>(HassRequestKind::DeviceRegistry),
            hass_client.request::<EntityRegistryList<'_>>(HassRequestKind::EntityRegistry),
            hass_client.request::<StatesList<'_>>(HassRequestKind::GetStates),
        );

        let rooms = &rooms.get().0;
        let states = states.get();
        let devices = &devices.get().0;
        let entities = &entities.get().0;

        let all_entities = entities
            .iter()
            .filter_map(|v| v.device_id.as_deref().zip(Some(v)))
            .into_group_map();

        let room_devices = devices
            .iter()
            .filter_map(|v| v.area_id.as_deref().zip(all_entities.get(v.id.as_ref())))
            .into_group_map();

        let rooms = rooms
            .iter()
            .map(|room| build_room(&room_devices, room))
            .collect();

        eprintln!("{rooms:#?}");

        let mut media_players = BTreeMap::new();
        let mut lights = BTreeMap::new();

        for state in &states.0 {
            match &state.attributes {
                StateAttributes::MediaPlayer(attr) => {
                    media_players.insert(
                        Intern::<str>::from(state.entity_id.as_ref()).as_ref(),
                        MediaPlayer::new(attr, &state.state, &hass_client.base),
                    );
                }
                StateAttributes::Light(attr) => {
                    lights.insert(
                        Intern::<str>::from(state.entity_id.as_ref()).as_ref(),
                        Light::from((attr.clone(), state.state.as_ref())),
                    );
                }
                _ => {}
            }
        }

        let (entity_updates, _) = broadcast::channel(10);

        let this = Arc::new(Self {
            client: hass_client,
            rooms,
            weather: Atomic::new(Weather::parse_from_states(states)),
            media_players: Mutex::new(media_players),
            lights: Mutex::new(lights),
            entity_updates: entity_updates.clone(),
        });

        this.clone().spawn_worker();

        this
    }

    pub fn rooms(&self) -> impl Iterator<Item = (&'static str, &'_ Room)> + '_ {
        self.rooms.iter().map(|(k, v)| (*k, v))
    }

    pub fn room(&self, id: &str) -> &Room {
        self.rooms.get(id).unwrap()
    }

    pub fn current_weather(&self) -> Weather {
        self.weather.load(Ordering::Acquire)
    }

    pub fn subscribe_weather(&self) -> impl Stream<Item = ()> {
        BroadcastStream::new(self.entity_updates.subscribe())
            .filter_map(|v| future::ready(v.ok()))
            .filter(|v| future::ready(v.starts_with("weather.")))
            .map(|_| ())
    }

    pub fn subscribe_id(&self, id: &'static str) -> impl Stream<Item = ()> {
        BroadcastStream::new(self.entity_updates.subscribe())
            .filter_map(|v| future::ready(v.ok()))
            .filter(move |v| future::ready(&**v == id))
            .map(|_| ())
    }

    pub fn fetch_light(&self, entity_id: &'static str) -> Option<Light> {
        self.lights.lock().get(entity_id).cloned()
    }

    pub fn speaker(&self, speaker_id: &'static str) -> EloquentSpeaker<'_> {
        EloquentSpeaker {
            speaker_id,
            oracle: self,
        }
    }

    pub async fn set_light_state(&self, entity_id: &'static str, on: bool) {
        let _res = self
            .client
            .call_service(
                entity_id,
                CallServiceRequestData::Light(if on {
                    CallServiceRequestLight::TurnOn(CallServiceRequestLightTurnOn {
                        brightness: None,
                        hs_color: None,
                    })
                } else {
                    CallServiceRequestLight::TurnOff
                }),
            )
            .await;
    }

    pub async fn update_light(
        &self,
        entity_id: &'static str,
        hue: f32,
        saturation: f32,
        brightness: f32,
    ) {
        let _res = self
            .client
            .call_service(
                entity_id,
                CallServiceRequestData::Light(CallServiceRequestLight::TurnOn(
                    CallServiceRequestLightTurnOn {
                        hs_color: Some((hue, saturation * 100.)),
                        brightness: Some(clamp_to_u8(brightness)),
                    },
                )),
            )
            .await;
    }

    pub fn spawn_worker(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut recv = self.client.subscribe();
            let mut second_tick = tokio::time::interval(Duration::from_secs(1));
            second_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

            let mut active_media_players = self
                .media_players
                .lock()
                .iter()
                .filter(|(_k, v)| v.is_playing())
                .map(|(k, _v)| *k)
                .collect::<HashSet<_>>();

            loop {
                tokio::select! {
                    msg = recv.recv() => match msg {
                        Ok(msg) => self.handle_state_update_event(&msg, &mut active_media_players),
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    },
                    _ = second_tick.tick(), if !active_media_players.is_empty() => {
                        self.update_media_player_positions(&active_media_players);
                    },
                }
            }
        });
    }

    fn update_media_player_positions(&self, active_media_players: &HashSet<&'static str>) {
        let mut media_players = self.media_players.lock();

        for entity_id in active_media_players {
            let Some(MediaPlayer::Speaker(speaker)) = media_players.get_mut(entity_id) else {
                continue;
            };

            speaker.actual_media_position = speaker
                .media_position
                .zip(speaker.media_position_updated_at)
                .map(calculate_actual_media_position);

            let _res = self.entity_updates.send(Arc::from(*entity_id));
        }
    }

    fn handle_state_update_event(
        &self,
        msg: &Yoke<Event<'static>, String>,
        active_media_players: &mut HashSet<&'static str>,
    ) {
        match msg.get() {
            Event::StateChanged(state_changed) => {
                match &state_changed.new_state.attributes {
                    StateAttributes::MediaPlayer(attrs) => {
                        let entity_id =
                            Intern::<str>::from(state_changed.entity_id.as_ref()).as_ref();
                        let new_state = MediaPlayer::new(
                            attrs,
                            &state_changed.new_state.state,
                            &self.client.base,
                        );

                        if new_state.is_playing() {
                            active_media_players.insert(entity_id);
                        } else {
                            active_media_players.remove(entity_id);
                        }

                        self.media_players.lock().insert(entity_id, new_state);
                    }
                    StateAttributes::Weather(attrs) => {
                        self.weather.store(
                            Weather::parse_from_state_and_attributes(
                                state_changed.new_state.state.as_ref(),
                                attrs,
                            ),
                            Ordering::Release,
                        );
                    }
                    StateAttributes::Light(attrs) => {
                        self.lights.lock().insert(
                            Intern::<str>::from(state_changed.entity_id.as_ref()).as_ref(),
                            Light::from((attrs.clone(), state_changed.new_state.state.as_ref())),
                        );
                    }
                    _ => {
                        // TODO
                    }
                }

                let _res = self
                    .entity_updates
                    .send(Arc::from(state_changed.entity_id.as_ref()));
            }
        }
    }
}

/// Eloquent interface for interacting with a speaker. Does not hold any state
/// of its own.
pub struct EloquentSpeaker<'a> {
    oracle: &'a Oracle,
    speaker_id: &'static str,
}

impl EloquentSpeaker<'_> {
    async fn call(&self, msg: CallServiceRequestMediaPlayer) {
        let _res = self
            .oracle
            .client
            .call_service(self.speaker_id, CallServiceRequestData::MediaPlayer(msg))
            .await;
    }

    pub async fn set_mute(&self, is_volume_muted: bool) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.muted = true;
        }

        self.call(CallServiceRequestMediaPlayer::VolumeMute(
            CallServiceRequestMediaPlayerVolumeMute { is_volume_muted },
        ))
        .await;
    }

    pub async fn set_volume(&self, volume_level: f32) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.volume = volume_level;
        }

        self.call(CallServiceRequestMediaPlayer::VolumeSet(
            CallServiceRequestMediaPlayerVolumeSet { volume_level },
        ))
        .await;
    }

    pub async fn seek(&self, position: Duration) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.media_position = Some(position);
            speaker.actual_media_position = Some(position);
            speaker.media_position_updated_at = Some(OffsetDateTime::now_utc());
        }

        self.call(CallServiceRequestMediaPlayer::MediaSeek(
            CallServiceRequestMediaPlayerMediaSeek {
                seek_position: position,
            },
        ))
        .await;
    }

    pub async fn set_shuffle(&self, shuffle: bool) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.shuffle = shuffle;
        }

        self.call(CallServiceRequestMediaPlayer::ShuffleSet(
            CallServiceRequestMediaPlayerShuffleSet { shuffle },
        ))
        .await;
    }

    pub async fn set_repeat(&self, repeat: MediaPlayerRepeat) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.repeat = repeat;
        }

        self.call(CallServiceRequestMediaPlayer::RepeatSet(
            CallServiceRequestMediaPlayerRepeatSet { repeat },
        ))
        .await;
    }

    pub async fn play(&self) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.state = MediaPlayerSpeakerState::Playing;
        }

        self.call(CallServiceRequestMediaPlayer::MediaPlay).await;
    }

    pub async fn pause(&self) {
        if let MediaPlayer::Speaker(speaker) = self
            .oracle
            .media_players
            .lock()
            .get_mut(self.speaker_id)
            .unwrap()
        {
            speaker.state = MediaPlayerSpeakerState::Paused;
        }

        self.call(CallServiceRequestMediaPlayer::MediaPause).await;
    }

    pub async fn next(&self) {
        self.call(CallServiceRequestMediaPlayer::MediaNextTrack)
            .await;
    }

    pub async fn previous(&self) {
        self.call(CallServiceRequestMediaPlayer::MediaPreviousTrack)
            .await;
    }
}

fn build_room(
    room_devices: &HashMap<&str, Vec<&Vec<&Entity>>>,
    room: &Area,
) -> (&'static str, Room) {
    let entities = room_devices
        .get(room.area_id.as_ref())
        .iter()
        .flat_map(|v| v.iter())
        .flat_map(|v| v.iter())
        .map(|v| Intern::from(v.entity_id.as_ref()))
        .collect::<Vec<Intern<str>>>();

    let speaker_id = entities
        .iter()
        .filter(|v| {
            // TODO: support multiple media players in one room
            v.as_ref() != "media_player.lg_webos_smart_tv"
        })
        .find(|v| v.starts_with("media_player."))
        .copied();

    let lights = entities
        .iter()
        .filter(|v| v.starts_with("light."))
        .copied()
        .collect();

    let area = Intern::<str>::from(room.area_id.as_ref()).as_ref();
    let room = Room {
        name: Intern::from(room.name.as_ref()),
        entities,
        speaker_id,
        lights,
    };

    (area, room)
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum MediaPlayer {
    Speaker(MediaPlayerSpeaker),
    Tv(MediaPlayerTv),
}

impl MediaPlayer {
    pub fn is_playing(&self) -> bool {
        if let MediaPlayer::Speaker(speaker) = self {
            speaker.state == MediaPlayerSpeakerState::Playing
        } else {
            false
        }
    }
}

impl MediaPlayer {
    fn new(attr: &StateMediaPlayerAttributes, state: &str, base: &Url) -> Self {
        let state = match state {
            "playing" => MediaPlayerSpeakerState::Playing,
            "paused" => MediaPlayerSpeakerState::Paused,
            "idle" => MediaPlayerSpeakerState::Idle,
            "unavailable" => MediaPlayerSpeakerState::Unavailable,
            "off" => MediaPlayerSpeakerState::Off,
            v => panic!("unknown speaker state: {v}"),
        };

        let repeat = match attr.repeat.as_deref() {
            None | Some("off") => MediaPlayerRepeat::Off,
            Some("all") => MediaPlayerRepeat::All,
            Some("one") => MediaPlayerRepeat::One,
            v => panic!("unknown speaker repeat: {v:?}"),
        };

        if attr.volume_level.is_some() {
            let actual_media_position = attr
                .media_position
                .map(Duration::from_secs)
                .zip(attr.media_position_updated_at)
                .map(calculate_actual_media_position);

            MediaPlayer::Speaker(MediaPlayerSpeaker {
                state,
                volume: attr.volume_level.unwrap(),
                muted: attr.is_volume_muted.unwrap(),
                source: Box::from(attr.source.as_deref().unwrap_or("")),
                actual_media_position,
                media_duration: attr.media_duration.map(Duration::from_secs),
                media_position: attr.media_position.map(Duration::from_secs),
                media_position_updated_at: attr.media_position_updated_at,
                media_title: attr.media_title.as_deref().map(Box::from),
                media_artist: attr.media_artist.as_deref().map(Box::from),
                media_album_name: attr.media_album_name.as_deref().map(Box::from),
                shuffle: attr.shuffle.unwrap_or(false),
                repeat,
                entity_picture: attr
                    .entity_picture
                    .as_deref()
                    .map(|path| base.join(path).unwrap()),
            })
        } else {
            MediaPlayer::Tv(MediaPlayerTv {})
        }
    }
}

#[derive(Debug, Clone)]
pub struct Light {
    pub on: Option<bool>,
    pub min_color_temp_kelvin: Option<u16>,
    pub max_color_temp_kelvin: Option<u16>,
    pub min_mireds: Option<u16>,
    pub max_mireds: Option<u16>,
    pub supported_color_modes: Vec<ColorMode>,
    pub mode: Option<Box<str>>,
    pub dynamics: Option<Box<str>>,
    pub friendly_name: Box<str>,
    pub color_mode: Option<ColorMode>,
    pub brightness: Option<f32>,
    pub color_temp_kelvin: Option<u16>,
    pub color_temp: Option<u16>,
    pub hs_color: Option<(f32, f32)>,
}

impl From<(StateLightAttributes<'_>, &str)> for Light {
    fn from((value, state): (StateLightAttributes<'_>, &str)) -> Self {
        let on = match state {
            "on" => Some(true),
            "off" => Some(false),
            "unavailable" => None,
            v => panic!("unknown light state: {v}"),
        };

        Self {
            on,
            min_color_temp_kelvin: value.min_color_temp_kelvin,
            max_color_temp_kelvin: value.max_color_temp_kelvin,
            min_mireds: value.min_mireds,
            max_mireds: value.max_mireds,
            supported_color_modes: value.supported_color_modes.clone(),
            mode: value.mode.map(Cow::into_owned).map(Box::from),
            dynamics: value.dynamics.map(Cow::into_owned).map(Box::from),
            friendly_name: Box::from(value.friendly_name.as_ref()),
            color_mode: value.color_mode,
            brightness: value.brightness,
            color_temp_kelvin: value.color_temp_kelvin,
            color_temp: value.color_temp,
            hs_color: value.hs_color,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaPlayerSpeaker {
    pub state: MediaPlayerSpeakerState,
    pub volume: f32,
    pub muted: bool,
    pub source: Box<str>,
    pub media_duration: Option<Duration>,
    pub media_position: Option<Duration>,
    pub media_position_updated_at: Option<time::OffsetDateTime>,
    pub actual_media_position: Option<Duration>,
    pub media_title: Option<Box<str>>,
    pub media_artist: Option<Box<str>>,
    pub media_album_name: Option<Box<str>>,
    pub shuffle: bool,
    pub repeat: MediaPlayerRepeat,
    pub entity_picture: Option<Url>,
}

fn calculate_actual_media_position(
    (position, updated_at): (Duration, time::OffsetDateTime),
) -> Duration {
    let now = time::OffsetDateTime::now_utc();
    let since_update = now - updated_at;

    (position + since_update).unsigned_abs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaPlayerSpeakerState {
    Playing,
    Unavailable,
    Off,
    Idle,
    Paused,
}

impl MediaPlayerSpeakerState {
    pub fn is_playing(self) -> bool {
        matches!(self, MediaPlayerSpeakerState::Playing)
    }
}

#[derive(Debug, Clone)]
pub struct MediaPlayerTv {}

#[derive(Debug, Clone)]
pub struct Room {
    pub name: Intern<str>,
    pub entities: Vec<Intern<str>>,
    pub speaker_id: Option<Intern<str>>,
    pub lights: BTreeSet<Intern<str>>,
}

impl Room {
    pub fn speaker(&self, oracle: &Oracle) -> Option<(&'static str, MediaPlayerSpeaker)> {
        match self.speaker_id.and_then(|v| {
            oracle
                .media_players
                .lock()
                .get(v.as_ref())
                .cloned()
                .zip(Some(v))
        })? {
            (MediaPlayer::Speaker(v), id) => Some((id.as_ref(), v)),
            (MediaPlayer::Tv(_), _) => None,
        }
    }

    pub fn lights(&self, oracle: &Oracle) -> BTreeMap<&'static str, Light> {
        let lights = oracle.lights.lock();

        self.lights
            .iter()
            .filter_map(|v| Some((*v).as_ref()).zip(lights.get(v.as_ref()).cloned()))
            .collect()
    }
}

#[derive(Debug, Copy, Clone, NoUninit)]
#[repr(C)]
pub struct Weather {
    pub temperature: i16,
    pub high: i16,
    pub low: i16,
    pub condition: u16,
}

impl Weather {
    pub fn weather_condition(self) -> WeatherCondition {
        WeatherCondition::from_repr(self.condition).unwrap_or_default()
    }

    #[allow(clippy::cast_possible_truncation)]
    fn parse_from_state_and_attributes(state: &str, attributes: &StateWeatherAttributes) -> Self {
        let condition = WeatherCondition::from_str(state).unwrap_or_default();

        let (high, low) =
            attributes
                .forecast
                .iter()
                .fold((i16::MIN, i16::MAX), |(high, low), curr| {
                    let temp = curr.temperature.round() as i16;

                    (high.max(temp), low.min(temp))
                });

        Self {
            temperature: attributes.temperature.round() as i16,
            condition: condition as u16,
            high,
            low,
        }
    }

    fn parse_from_states(states: &StatesList) -> Self {
        let (state, attrs) = states
            .0
            .iter()
            .find_map(|v| match &v.attributes {
                StateAttributes::Weather(attr) => Some((&v.state, attr)),
                _ => None,
            })
            .unwrap();

        Self::parse_from_state_and_attributes(state.as_ref(), attrs)
    }
}
