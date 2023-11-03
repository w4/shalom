use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use iced::futures::{future, Stream, StreamExt};
use internment::Intern;
use itertools::Itertools;
use tokio::sync::{broadcast, broadcast::error::RecvError};
use tokio_stream::wrappers::BroadcastStream;
use url::Url;

use crate::{
    hass_client::{
        responses::{
            Area, AreaRegistryList, CallServiceResponse, ColorMode, DeviceRegistryList, Entity,
            EntityRegistryList, StateAttributes, StateLightAttributes, StateMediaPlayerAttributes,
            StateWeatherAttributes, StatesList, WeatherCondition,
        },
        CallServiceRequest, CallServiceRequestData, CallServiceRequestLight,
        CallServiceRequestLightTurnOn, CallServiceRequestTarget, Event, HassRequestKind,
    },
    widgets::colour_picker::clamp_to_u8,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<&'static str, Room>,
    weather: Mutex<Weather>,
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
                        MediaPlayer::new(attr, &hass_client.base),
                    );
                }
                StateAttributes::Light(attr) => {
                    lights.insert(
                        Intern::<str>::from(state.entity_id.as_ref()).as_ref(),
                        Light::from(attr.clone()),
                    );
                }
                _ => {}
            }
        }

        let (entity_updates, _) = broadcast::channel(10);

        let this = Arc::new(Self {
            client: hass_client,
            rooms,
            weather: Mutex::new(Weather::parse_from_states(states)),
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
        *self.weather.lock().unwrap()
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
        self.lights.lock().unwrap().get(entity_id).cloned()
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
            .request::<CallServiceResponse>(HassRequestKind::CallService(CallServiceRequest {
                target: Some(CallServiceRequestTarget { entity_id }),
                data: CallServiceRequestData::Light(CallServiceRequestLight::TurnOn(
                    CallServiceRequestLightTurnOn {
                        hs_color: (hue, saturation * 100.),
                        brightness: clamp_to_u8(brightness),
                    },
                )),
            }))
            .await;
    }

    pub fn spawn_worker(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut recv = self.client.subscribe();

            loop {
                let msg = match recv.recv().await {
                    Ok(msg) => msg,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                };

                match msg.get() {
                    Event::StateChanged(state_changed) => {
                        match &state_changed.new_state.attributes {
                            StateAttributes::MediaPlayer(attrs) => {
                                self.media_players.lock().unwrap().insert(
                                    Intern::<str>::from(state_changed.entity_id.as_ref()).as_ref(),
                                    MediaPlayer::new(attrs, &self.client.base),
                                );
                            }
                            StateAttributes::Weather(attrs) => {
                                *self.weather.lock().unwrap() =
                                    Weather::parse_from_state_and_attributes(
                                        state_changed.new_state.state.as_ref(),
                                        attrs,
                                    );
                            }
                            StateAttributes::Light(attrs) => {
                                self.lights.lock().unwrap().insert(
                                    Intern::<str>::from(state_changed.entity_id.as_ref()).as_ref(),
                                    Light::from(attrs.clone()),
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
        });
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
    fn new(attr: &StateMediaPlayerAttributes, base: &Url) -> Self {
        if attr.volume_level.is_some() {
            MediaPlayer::Speaker(MediaPlayerSpeaker {
                volume: attr.volume_level.unwrap(),
                muted: attr.is_volume_muted.unwrap(),
                source: Box::from(attr.source.as_deref().unwrap_or("")),
                media_duration: attr.media_duration.map(Duration::from_secs),
                media_position: attr.media_position.map(Duration::from_secs),
                media_title: attr.media_title.as_deref().map(Box::from),
                media_artist: attr.media_artist.as_deref().map(Box::from),
                media_album_name: attr.media_album_name.as_deref().map(Box::from),
                shuffle: attr.shuffle.unwrap_or(false),
                repeat: Box::from(attr.repeat.as_deref().unwrap_or("")),
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

impl From<StateLightAttributes<'_>> for Light {
    fn from(value: StateLightAttributes<'_>) -> Self {
        Self {
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
    pub volume: f32,
    pub muted: bool,
    pub source: Box<str>,
    pub media_duration: Option<Duration>,
    pub media_position: Option<Duration>,
    pub media_title: Option<Box<str>>,
    pub media_artist: Option<Box<str>>,
    pub media_album_name: Option<Box<str>>,
    pub shuffle: bool,
    pub repeat: Box<str>,
    pub entity_picture: Option<Url>,
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
    pub fn speaker(&self, oracle: &Oracle) -> Option<MediaPlayerSpeaker> {
        match self.speaker_id.and_then(|v| {
            oracle
                .media_players
                .lock()
                .unwrap()
                .get(v.as_ref())
                .cloned()
        })? {
            MediaPlayer::Speaker(v) => Some(v),
            MediaPlayer::Tv(_) => None,
        }
    }

    pub fn light_names(&self, oracle: &Oracle) -> BTreeMap<&'static str, Box<str>> {
        let lights = oracle.lights.lock().unwrap();

        self.lights
            .iter()
            .filter_map(|v| Some((*v).as_ref()).zip(lights.get(v.as_ref())))
            .map(|(id, light)| {
                eprintln!("{light:?}");
                (id, light.friendly_name.clone())
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Weather {
    pub temperature: i16,
    pub high: i16,
    pub low: i16,
    pub condition: WeatherCondition,
}

impl Weather {
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
            condition,
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
