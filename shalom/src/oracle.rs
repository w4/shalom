use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    time::Duration,
};

use internment::Intern;
use url::Url;

use crate::hass_client::{
    responses::{
        AreaRegistryList, DeviceRegistryList, EntityRegistryList, StateAttributes, StatesList,
        WeatherCondition,
    },
    HassRequestKind,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Oracle {
    client: crate::hass_client::Client,
    rooms: BTreeMap<&'static str, Room>,
    pub weather: Weather,
    pub media_players: BTreeMap<&'static str, MediaPlayer>,
}

impl Oracle {
    pub async fn new(hass_client: crate::hass_client::Client) -> Self {
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
            .fold(HashMap::<_, Vec<_>>::new(), |mut acc, curr| {
                if let Some(device_id) = curr.device_id.as_deref() {
                    acc.entry(device_id).or_default().push(curr);
                }

                acc
            });

        let room_devices = devices
            .iter()
            .fold(HashMap::<_, Vec<_>>::new(), |mut acc, curr| {
                if let (Some(area_id), Some(entity)) =
                    (curr.area_id.as_deref(), all_entities.get(curr.id.as_ref()))
                {
                    acc.entry(area_id).or_default().push(entity);
                }

                acc
            });

        let rooms = rooms
            .iter()
            .map(|room| {
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
            })
            .collect();

        eprintln!("{rooms:#?}");

        let media_players = states
            .0
            .iter()
            .filter_map(|state| {
                if let StateAttributes::MediaPlayer(attr) = &state.attributes {
                    let kind = if attr.volume_level.is_some() {
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
                                .map(|path| hass_client.base.join(path).unwrap()),
                        })
                    } else {
                        MediaPlayer::Tv(MediaPlayerTv {})
                    };

                    Some((Intern::<str>::from(state.entity_id.as_ref()).as_ref(), kind))
                } else {
                    None
                }
            })
            .collect();

        Self {
            client: hass_client,
            rooms,
            weather: Weather::parse_from_states(states),
            media_players,
        }
    }

    pub fn rooms(&self) -> impl Iterator<Item = (&'static str, &'_ Room)> + '_ {
        self.rooms.iter().map(|(k, v)| (*k, v))
    }

    pub fn room(&self, id: &str) -> &Room {
        self.rooms.get(id).unwrap()
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum MediaPlayer {
    Speaker(MediaPlayerSpeaker),
    Tv(MediaPlayerTv),
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

#[derive(Debug)]
pub struct MediaPlayerTv {}

#[derive(Debug, Clone)]
pub struct Room {
    pub name: Intern<str>,
    pub entities: Vec<Intern<str>>,
    pub speaker_id: Option<Intern<str>>,
}

impl Room {
    pub fn speaker<'a>(&self, oracle: &'a Oracle) -> Option<&'a MediaPlayerSpeaker> {
        match self
            .speaker_id
            .and_then(|v| oracle.media_players.get(v.as_ref()))?
        {
            MediaPlayer::Speaker(v) => Some(v),
            MediaPlayer::Tv(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct Weather {
    pub temperature: i16,
    pub high: i16,
    pub low: i16,
    pub condition: WeatherCondition,
}

impl Weather {
    #[allow(clippy::cast_possible_truncation)]
    fn parse_from_states(states: &StatesList) -> Self {
        let (state, weather) = states
            .0
            .iter()
            .find_map(|v| match &v.attributes {
                StateAttributes::Weather(attr) => Some((&v.state, attr)),
                _ => None,
            })
            .unwrap();

        let condition = WeatherCondition::from_str(state).unwrap_or_default();

        let (high, low) =
            weather
                .forecast
                .iter()
                .fold((i16::MIN, i16::MAX), |(high, low), curr| {
                    let temp = curr.temperature.round() as i16;

                    (high.max(temp), low.min(temp))
                });

        Self {
            temperature: weather.temperature.round() as i16,
            condition,
            high,
            low,
        }
    }
}
