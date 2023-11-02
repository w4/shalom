#![allow(clippy::forget_non_drop, dead_code)]

use std::{collections::HashMap, sync::Arc, time::Duration};

use iced::futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use url::Url;
use yoke::{Yoke, Yokeable};

use crate::config::HomeAssistantConfig;

#[derive(Clone, Debug)]
pub struct Client {
    pub base: url::Url,
    sender: mpsc::Sender<(
        HassRequestKind,
        oneshot::Sender<Yoke<&'static RawValue, String>>,
    )>,
    broadcast_channel: broadcast::Sender<Arc<Yoke<Event<'static>, String>>>,
}

impl Client {
    pub async fn request<T: for<'a> Yokeable<'a>>(
        &self,
        request: HassRequestKind,
    ) -> Yoke<T, String>
    where
        for<'a> <T as Yokeable<'a>>::Output: Deserialize<'a>,
    {
        let (send, recv) = oneshot::channel();
        self.sender.send((request, send)).await.unwrap();
        let resp = recv.await.unwrap();

        resp.map_project(move |value, _| serde_json::from_str(value.get()).unwrap())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Yoke<Event<'static>, String>>> {
        self.broadcast_channel.subscribe()
    }
}

pub async fn create(config: HomeAssistantConfig) -> Client {
    let (sender, mut recv) = mpsc::channel(10);

    let uri = format!("wss://{}/api/websocket", config.uri);
    let (mut connection, _response) = tokio_tungstenite::connect_async(&uri).await.unwrap();

    let (ready_send, ready_recv) = oneshot::channel();
    let mut ready_send = Some(ready_send);

    let (broadcast_channel, _broadcast_recv) = broadcast::channel(10);

    let broadcast_send = broadcast_channel.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        let mut counter: u64 = 0;
        let mut pending: HashMap<u64, oneshot::Sender<Yoke<&'static RawValue, String>>> =
            HashMap::new();

        loop {
            tokio::select! {
                Some(message) = connection.next() => {
                    let message = message.unwrap();

                    #[allow(clippy::match_same_arms)]
                    match message {
                        Message::Pong(ts) => {
                            let ts = i128::from_be_bytes(ts.try_into().unwrap());
                            let ts = OffsetDateTime::from_unix_timestamp_nanos(ts).unwrap();

                            eprintln!("rtt: {}", OffsetDateTime::now_utc() - ts);
                        }
                        Message::Text(payload) => {
                            let yoked_payload: Yoke<HassResponse, String> = Yoke::attach_to_cart(payload, |s| serde_json::from_str(s).unwrap());

                            let payload: &HassResponse = yoked_payload.get();

                            match payload.type_ {
                                HassResponseType::AuthRequired => {
                                    let payload = HassRequest {
                                        id: None,
                                        inner: HassRequestKind::Auth {
                                            access_token: config.token.clone(),
                                        }
                                    }
                                    .to_request();

                                    connection
                                        .send(payload)
                                        .await
                                        .unwrap();
                                }
                                HassResponseType::AuthInvalid => {
                                    eprintln!("invalid auth");
                                }
                                HassResponseType::AuthOk => {
                                    ready_send.take().unwrap().send(()).unwrap();

                                    counter += 1;
                                    let counter = counter;

                                    connection
                                        .send(HassRequest {
                                            id: Some(counter),
                                            inner: HassRequestKind::SubscribeEvents {
                                                event_type: Some("state_changed".to_string()),
                                            },
                                        }.to_request())
                                        .await
                                        .unwrap();
                                }
                                HassResponseType::Result => {
                                    let id = payload.id.unwrap();
                                    let payload = yoked_payload.try_map_project(move |yk, _| yk.result.ok_or(()));

                                    if let (Some(channel), Ok(payload)) = (pending.remove(&id), payload) {
                                        let _res = channel.send(payload);
                                    }
                                }
                                HassResponseType::Event => {
                                    let payload = yoked_payload.map_project(move |yk, _| yk.event.unwrap());
                                    let _res = broadcast_send.send(Arc::new(payload));
                                }
                            }
                        }
                        Message::Close(_) => {
                            // eprintln!("Reconnecting...");
                            // connection = tokio_tungstenite::connect_async(&uri).await.unwrap().0;
                        }
                        _ => {}
                    }
                }
                Some((inner, reply)) = recv.recv() => {
                    counter += 1;
                    let counter = counter;

                    connection.send(HassRequest {
                        id: Some(counter),
                        inner,
                    }.to_request()).await.unwrap();

                    pending.insert(counter, reply);
                }
                _ = interval.tick() => {
                    connection.send(Message::Ping(OffsetDateTime::now_utc().unix_timestamp_nanos().to_be_bytes().to_vec())).await.unwrap();
                }
            }
        }
    });

    ready_recv.await.unwrap();

    Client {
        base: Url::parse(&format!("https://{}/", config.uri)).unwrap(),
        sender,
        broadcast_channel,
    }
}

#[derive(Deserialize, Yokeable, Debug)]
struct HassResponse<'a> {
    id: Option<u64>,
    #[serde(rename = "type")]
    type_: HassResponseType,
    #[serde(borrow)]
    result: Option<&'a RawValue>,
    #[serde(borrow, bound(deserialize = "'a: 'de"))]
    event: Option<Event<'a>>,
}

#[derive(Deserialize, Clone, Debug, Yokeable)]
#[serde(rename_all = "snake_case", tag = "event_type", content = "data")]
pub enum Event<'a> {
    StateChanged(#[serde(borrow, bound(deserialize = "'a: 'de"))] events::StateChanged<'a>),
}

#[derive(Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HassResponseType {
    AuthRequired,
    AuthOk,
    AuthInvalid,
    Result,
    Event,
}

#[derive(Serialize)]
struct HassRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    #[serde(flatten)]
    inner: HassRequestKind,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HassRequestKind {
    Auth {
        access_token: String,
    },
    GetStates,
    #[serde(rename = "config/area_registry/list")]
    AreaRegistry,
    #[serde(rename = "config/entity_registry/list")]
    EntityRegistry,
    #[serde(rename = "config/device_registry/list")]
    DeviceRegistry,
    SubscribeEvents {
        event_type: Option<String>,
    },
}

impl HassRequest {
    pub fn to_request(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}

pub mod events {
    use std::borrow::Cow;

    use serde::Deserialize;

    #[derive(Deserialize, Clone, Debug)]
    pub struct StateChanged<'a> {
        #[serde(borrow)]
        pub entity_id: Cow<'a, str>,
        #[serde(borrow, bound(deserialize = "'a: 'de"))]
        pub old_state: super::responses::State<'a>,
        #[serde(borrow, bound(deserialize = "'a: 'de"))]
        pub new_state: super::responses::State<'a>,
    }
}

pub mod responses {
    use std::{
        borrow::Cow,
        fmt::{Display, Formatter},
    };

    use serde::{
        de,
        de::{MapAccess, Visitor},
        Deserialize, Deserializer,
    };
    use serde_json::value::RawValue;
    use strum::EnumString;
    use yoke::Yokeable;

    use crate::theme::Icon;

    #[derive(Deserialize, Yokeable, Debug)]
    pub struct AreaRegistryList<'a>(#[serde(borrow)] pub Vec<Area<'a>>);

    #[derive(Deserialize, Debug)]
    pub struct Area<'a> {
        #[serde(borrow)]
        pub aliases: Vec<Cow<'a, str>>,
        #[serde(borrow)]
        pub area_id: Cow<'a, str>,
        #[serde(borrow)]
        pub name: Cow<'a, str>,
        #[serde(borrow)]
        pub picture: Option<Cow<'a, str>>,
    }

    #[derive(Deserialize, Yokeable, Debug)]
    pub struct DeviceRegistryList<'a>(#[serde(borrow)] pub Vec<Device<'a>>);

    #[derive(Deserialize, Debug)]
    pub struct Device<'a> {
        #[serde(borrow)]
        pub area_id: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub configuration_url: Option<Cow<'a, str>>,
        #[serde(borrow, default)]
        pub config_entries: Vec<Cow<'a, str>>,
        #[serde(borrow)]
        pub connections: Vec<Vec<Cow<'a, str>>>,
        #[serde(borrow)]
        pub disabled_by: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub entry_type: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub hw_version: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub id: Cow<'a, str>,
        #[serde(borrow, default)]
        pub identifiers: Vec<Vec<Cow<'a, str>>>,
        #[serde(borrow)]
        pub manufacturer: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub model: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub name_by_user: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub sw_version: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub via_device_id: Option<Cow<'a, str>>,
    }

    #[derive(Deserialize, Yokeable, Debug)]
    pub struct EntityRegistryList<'a>(#[serde(borrow)] pub Vec<Entity<'a>>);

    #[derive(Deserialize, Debug)]
    pub struct Entity<'a> {
        #[serde(borrow)]
        pub area_id: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub config_entry_id: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub device_id: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub disabled_by: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub entity_category: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub entity_id: Cow<'a, str>,
        pub has_entity_name: bool,
        #[serde(borrow)]
        pub hidden_by: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub icon: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub id: Cow<'a, str>,
        #[serde(borrow)]
        pub name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub original_name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub platform: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub translation_key: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub unique_id: Option<Cow<'a, str>>,
    }

    #[derive(Yokeable, Debug, Deserialize)]
    pub struct StatesList<'a>(#[serde(borrow, bound(deserialize = "'a: 'de"))] pub Vec<State<'a>>);

    #[derive(Debug, Clone)]
    pub struct State<'a> {
        pub entity_id: Cow<'a, str>,
        pub state: Cow<'a, str>,
        pub attributes: StateAttributes<'a>,
    }

    impl<'de> Deserialize<'de> for State<'de> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_struct(
                "State",
                &["entity_id", "state", "attributes"],
                StateVisitor {},
            )
        }
    }

    pub struct StateVisitor {}

    impl<'de> Visitor<'de> for StateVisitor {
        type Value = State<'de>;

        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("states struct")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut entity_id: Option<Cow<'de, str>> = None;
            let mut state: Option<Cow<'de, str>> = None;
            let mut attributes: Option<&'de RawValue> = None;

            while let Some(key) = map.next_key()? {
                match key {
                    "entity_id" => {
                        entity_id = Some(map.next_value()?);
                    }
                    "state" => {
                        state = Some(map.next_value()?);
                    }
                    "attributes" => {
                        attributes = Some(map.next_value()?);
                    }
                    _ => {
                        let _: &'de RawValue = map.next_value()?;
                    }
                }
            }

            let entity_id = entity_id.ok_or_else(|| de::Error::missing_field("entity_id"))?;
            let state = state.ok_or_else(|| de::Error::missing_field("state"))?;
            let attributes = attributes.ok_or_else(|| de::Error::missing_field("attributes"))?;

            let Some((kind, _)) = entity_id.split_once('.') else {
                return Err(de::Error::custom("invalid entity_id"));
            };

            let attributes = match kind {
                "sun" => StateAttributes::Sun(serde_json::from_str(attributes.get()).unwrap()),
                "media_player" => {
                    StateAttributes::MediaPlayer(serde_json::from_str(attributes.get()).unwrap())
                }
                "camera" => {
                    StateAttributes::Camera(serde_json::from_str(attributes.get()).unwrap())
                }
                "weather" => {
                    StateAttributes::Weather(serde_json::from_str(attributes.get()).unwrap())
                }
                "light" => StateAttributes::Light(serde_json::from_str(attributes.get()).unwrap()),
                _ => StateAttributes::Unknown,
            };

            Ok(State {
                entity_id,
                state,
                attributes,
            })
        }
    }

    #[derive(Deserialize, Debug, Clone)]
    #[allow(clippy::large_enum_variant)]
    pub enum StateAttributes<'a> {
        Sun(StateSunAttributes),
        MediaPlayer(#[serde(borrow)] StateMediaPlayerAttributes<'a>),
        Camera(#[serde(borrow)] StateCameraAttributes<'a>),
        Weather(#[serde(borrow)] StateWeatherAttributes<'a>),
        Light(#[serde(borrow)] StateLightAttributes<'a>),
        Unknown,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    pub struct StateSunAttributes {
        // next_dawn: time::OffsetDateTime,
        // next_dusk: time::OffsetDateTime,
        // next_midnight: time::OffsetDateTime,
        // next_noon: time::OffsetDateTime,
        // next_rising: time::OffsetDateTime,
        // next_setting: time::OffsetDateTime,
        elevation: f32,
        azimuth: f32,
        rising: bool,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateMediaPlayerAttributes<'a> {
        #[serde(borrow, default)]
        pub source_list: Vec<Cow<'a, str>>,
        #[serde(borrow, default)]
        pub group_members: Vec<Cow<'a, str>>,
        pub volume_level: Option<f32>,
        pub is_volume_muted: Option<bool>,
        #[serde(borrow)]
        pub media_content_id: Option<MediaContentId<'a>>,
        #[serde(borrow)]
        pub media_content_type: Option<Cow<'a, str>>,
        pub media_duration: Option<u64>,
        pub media_position: Option<u64>,
        pub media_title: Option<Cow<'a, str>>,
        pub media_artist: Option<Cow<'a, str>>,
        pub media_album_name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub source: Option<Cow<'a, str>>,
        pub shuffle: Option<bool>,
        #[serde(borrow)]
        pub repeat: Option<Cow<'a, str>>,
        pub queue_position: Option<u32>,
        pub queue_size: Option<u32>,
        #[serde(borrow)]
        pub device_class: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub friendly_name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub entity_picture: Option<Cow<'a, str>>,
    }

    #[derive(Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum MediaContentId<'a> {
        Uri(#[serde(borrow)] Cow<'a, str>),
        Int(u32),
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateCameraAttributes<'a> {
        #[serde(borrow)]
        access_token: Cow<'a, str>,
        #[serde(borrow)]
        friendly_name: Cow<'a, str>,
        #[serde(borrow)]
        stream_source: Option<Cow<'a, str>>,
        #[serde(borrow)]
        still_image_url: Option<Cow<'a, str>>,
        #[serde(borrow)]
        name: Option<Cow<'a, str>>,
        #[serde(borrow)]
        id: Option<Cow<'a, str>>,
        #[serde(borrow)]
        entity_picture: Cow<'a, str>,
    }

    #[derive(Default, Deserialize, Debug, EnumString, Copy, Clone)]
    #[serde(rename_all = "kebab-case")]
    #[strum(serialize_all = "kebab-case")]
    pub enum WeatherCondition {
        ClearNight,
        Cloudy,
        Fog,
        Hail,
        Lightning,
        LightningRainy,
        #[serde(rename = "partlycloudy")]
        #[strum(serialize = "partlycloudy")]
        PartlyCloudy,
        Pouring,
        Rainy,
        Snowy,
        SnowyRainy,
        Sunny,
        Windy,
        WindyVariant,
        Exceptional,
        #[default]
        #[serde(other)]
        Unknown,
    }

    impl WeatherCondition {
        pub fn icon(self, day_time: bool) -> Option<Icon> {
            match self {
                WeatherCondition::ClearNight => Some(Icon::ClearNight),
                WeatherCondition::Cloudy => Some(Icon::Cloud),
                WeatherCondition::Fog => Some(Icon::Fog),
                WeatherCondition::Hail => Some(Icon::Hail),
                WeatherCondition::Lightning => Some(Icon::Thunderstorms),
                WeatherCondition::LightningRainy => Some(Icon::ThunderstormsRain),
                WeatherCondition::PartlyCloudy => Some(if day_time {
                    Icon::PartlyCloudyDay
                } else {
                    Icon::PartlyCloudyNight
                }),
                WeatherCondition::Pouring => Some(Icon::ExtremeRain),
                WeatherCondition::Rainy => Some(Icon::Rain),
                WeatherCondition::Snowy | WeatherCondition::SnowyRainy => Some(Icon::Snow),
                WeatherCondition::Sunny => Some(Icon::ClearDay),
                WeatherCondition::Windy | WeatherCondition::WindyVariant => Some(Icon::Wind),
                WeatherCondition::Exceptional | WeatherCondition::Unknown => None,
            }
        }
    }

    impl Display for WeatherCondition {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(match self {
                WeatherCondition::ClearNight => "Clear",
                WeatherCondition::Cloudy => "Cloudy",
                WeatherCondition::Fog => "Fog",
                WeatherCondition::Hail => "Hail",
                WeatherCondition::Lightning | WeatherCondition::LightningRainy => "Lightning",
                WeatherCondition::PartlyCloudy => "Partly Cloudy",
                WeatherCondition::Pouring => "Heavy Rain",
                WeatherCondition::Rainy => "Rain",
                WeatherCondition::Snowy | WeatherCondition::SnowyRainy => "Snow",
                WeatherCondition::Sunny => "Sunny",
                WeatherCondition::Windy | WeatherCondition::WindyVariant => "Windy",
                WeatherCondition::Exceptional => "Exceptional",
                WeatherCondition::Unknown => "Unknown",
            })
        }
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateWeatherAttributes<'a> {
        pub temperature: f32,
        pub dew_point: f32,
        #[serde(borrow)]
        pub temperature_unit: Cow<'a, str>,
        pub humidity: f32,
        pub cloud_coverage: f32,
        pub pressure: f32,
        #[serde(borrow)]
        pub pressure_unit: Cow<'a, str>,
        pub wind_bearing: f32,
        pub wind_speed: f32,
        #[serde(borrow)]
        pub wind_speed_unit: Cow<'a, str>,
        #[serde(borrow)]
        pub visibility_unit: Cow<'a, str>,
        #[serde(borrow)]
        pub precipitation_unit: Cow<'a, str>,
        #[serde(borrow)]
        pub forecast: Vec<StateWeatherAttributesForecast<'a>>,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateWeatherAttributesForecast<'a> {
        #[serde(borrow)]
        pub condition: Cow<'a, str>,
        // datetime: time::OffsetDateTime,
        pub wind_bearing: f32,
        pub temperature: f32,
        #[serde(rename = "templow")]
        pub temperature_low: f32,
        pub wind_speed: f32,
        pub precipitation: f32,
        pub humidity: f32,
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct StateLightAttributes<'a> {
        pub min_color_temp_kelvin: Option<u16>,
        pub max_color_temp_kelvin: Option<u16>,
        pub min_mireds: Option<u16>,
        pub max_mireds: Option<u16>,
        #[serde(default)]
        pub supported_color_modes: Vec<ColorMode>,
        #[serde(borrow)]
        pub mode: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub dynamics: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub friendly_name: Cow<'a, str>,
        pub color_mode: Option<ColorMode>,
        pub brightness: Option<f32>,
        pub color_temp_kelvin: Option<u16>,
        pub color_temp: Option<u16>,
        pub xy_color: Option<(f32, f32)>,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    #[serde(rename_all = "snake_case")]
    pub enum ColorMode {
        ColorTemp,
        Xy,
    }
}
