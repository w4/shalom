use std::{
    collections::{BTreeMap, HashMap},
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

use crate::hass_client::{
    responses::{
        Area, AreaRegistryList, DeviceRegistryList, Entity, EntityRegistryList, StateAttributes,
        StateMediaPlayerAttributes, StateWeatherAttributes, StatesList, WeatherCondition,
    },
    Event, HassRequestKind,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<&'static str, Room>,
    pub weather: Mutex<Weather>,
    pub media_players: Mutex<BTreeMap<&'static str, MediaPlayer>>,
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

        let media_players = states
            .0
            .iter()
            .filter_map(|state| {
                if let StateAttributes::MediaPlayer(attr) = &state.attributes {
                    let kind = MediaPlayer::new(attr, &hass_client.base);
                    Some((Intern::<str>::from(state.entity_id.as_ref()).as_ref(), kind))
                } else {
                    None
                }
            })
            .collect();

        let (entity_updates, _) = broadcast::channel(10);

        let this = Arc::new(Self {
            client: hass_client,
            rooms,
            weather: Mutex::new(Weather::parse_from_states(states)),
            media_players: Mutex::new(media_players),
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

    let area = Intern::<str>::from(room.area_id.as_ref()).as_ref();
    let room = Room {
        name: Intern::from(room.name.as_ref()),
        entities,
        speaker_id,
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
