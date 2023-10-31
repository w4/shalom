#![allow(clippy::forget_non_drop)]

use std::{collections::HashMap, time::Duration};

use iced::futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use yoke::{Yoke, Yokeable};

use crate::config::HomeAssistantConfig;

#[derive(Clone, Debug)]
pub struct Client {
    sender: mpsc::Sender<(
        HassRequestKind,
        oneshot::Sender<Yoke<&'static RawValue, String>>,
    )>,
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
}

pub async fn create(config: HomeAssistantConfig) -> Client {
    let (sender, mut recv) = mpsc::channel(10);

    let uri = format!("ws://{}/api/websocket", config.uri);
    let (mut connection, _response) = tokio_tungstenite::connect_async(&uri).await.unwrap();

    let (ready_send, ready_recv) = oneshot::channel();
    let mut ready_send = Some(ready_send);

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
                                }
                                HassResponseType::Result => {
                                    let id = payload.id.unwrap();
                                    let payload = yoked_payload.map_project(move |yk, _| yk.result.unwrap());
                                    pending.remove(&id).unwrap().send(payload).unwrap();
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

    Client { sender }
}

#[derive(Deserialize, Yokeable)]
struct HassResponse<'a> {
    id: Option<u64>,
    #[serde(rename = "type")]
    type_: HassResponseType,
    #[serde(borrow)]
    result: Option<&'a RawValue>,
}

#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum HassResponseType {
    AuthRequired,
    AuthOk,
    AuthInvalid,
    Result,
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
}

impl HassRequest {
    pub fn to_request(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}

pub mod responses {
    use std::borrow::Cow;

    use serde::Deserialize;
    use yoke::Yokeable;

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
        #[serde(borrow)]
        pub config_entries: Vec<Cow<'a, str>>,
        #[serde(borrow)]
        pub connections: Vec<(Cow<'a, str>, Cow<'a, str>)>,
        #[serde(borrow)]
        pub disabled_by: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub entry_type: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub hw_version: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub id: Cow<'a, str>,
        #[serde(borrow)]
        pub identifiers: Vec<(Cow<'a, str>, Cow<'a, str>)>,
        #[serde(borrow)]
        pub manufacturer: Cow<'a, str>,
        #[serde(borrow)]
        pub model: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub name_by_user: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub name: Cow<'a, str>,
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
        pub config_entry_id: Cow<'a, str>,
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
        pub original_name: Cow<'a, str>,
        #[serde(borrow)]
        pub platform: Cow<'a, str>,
        #[serde(borrow)]
        pub translation_key: Option<Cow<'a, str>>,
        #[serde(borrow)]
        pub unique_id: Option<Cow<'a, str>>,
    }

    #[derive(Deserialize, Yokeable, Debug)]
    pub struct StatesList<'a>(#[serde(borrow)] pub Vec<State<'a>>);

    #[derive(Deserialize, Debug)]
    pub enum State<'a> {
        Sun {
            #[serde(borrow)]
            state: Cow<'a, str>,
            attributes: StateSunAttributes,
        },
        MediaPlayer {
            #[serde(borrow)]
            state: Cow<'a, str>,
            #[serde(borrow)]
            attributes: StateMediaPlayerAttributes<'a>,
        },
        Camera {
            #[serde(borrow)]
            state: Cow<'a, str>,
            #[serde(borrow)]
            attributes: StateCameraAttributes<'a>,
        },
        Weather {
            #[serde(borrow)]
            state: Cow<'a, str>,
            #[serde(borrow)]
            attributes: StateWeatherAttributes<'a>,
        },
        Light {
            #[serde(borrow)]
            state: Cow<'a, str>,
            #[serde(borrow)]
            attributes: StateLightAttributes<'a>,
        }
    }

    #[derive(Deserialize, Debug)]
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

    #[derive(Deserialize, Debug)]
    pub struct StateMediaPlayerAttributes<'a> {
        #[serde(borrow)]
        source_list: Vec<Cow<'a, str>>,
        #[serde(borrow)]
        group_members: Vec<Cow<'a, str>>,
        volume_level: f32,
        is_volume_muted: bool,
        #[serde(borrow)]
        media_content_id: Cow<'a, str>,
        #[serde(borrow)]
        media_content_type: Cow<'a, str>,
        #[serde(borrow)]
        source: Cow<'a, str>,
        shuffle: bool,
        #[serde(borrow)]
        repeat: Cow<'a, str>,
        queue_position: u32,
        queue_size: u32,
        #[serde(borrow)]
        device_class: Cow<'a, str>,
        #[serde(borrow)]
        friendly_name: Cow<'a, str>,
    }

    #[derive(Deserialize, Debug)]
    pub struct StateCameraAttributes<'a> {
        #[serde(borrow)]
        access_token: Cow<'a, str>,
        #[serde(borrow)]
        friendly_name: Cow<'a, str>,
        #[serde(borrow)]
        stream_source: Cow<'a, str>,
        #[serde(borrow)]
        still_image_url: Cow<'a, str>,
        #[serde(borrow)]
        name: Cow<'a, str>,
        #[serde(borrow)]
        id: Cow<'a, str>,
        #[serde(borrow)]
        entity_picture: Cow<'a, str>,
    }

    #[derive(Deserialize, Debug)]
    pub struct StateWeatherAttributes<'a> {
        temperature: f32,
        dew_point: f32,
        #[serde(borrow)]
        temperature_unit: Cow<'a, str>,
        humidity: u8,
        cloud_coverage: u8,
        pressure: f32,
        #[serde(borrow)]
        pressure_unit: Cow<'a, str>,
        wind_bearing: f32,
        wind_speed: f32,
        #[serde(borrow)]
        wind_speed_unit: Cow<'a, str>,
        #[serde(borrow)]
        visibility_unit: Cow<'a, str>,
        #[serde(borrow)]
        precipitation_unit: Cow<'a, str>,
        #[serde(borrow)]
        forecast: Vec<StateWeatherAttributesForecast<'a>>,
    }

    #[derive(Deserialize, Debug)]
    pub struct StateWeatherAttributesForecast<'a> {
        #[serde(borrow)]
        condition: Cow<'a, str>,
        // datetime: time::OffsetDateTime,
        wind_bearing: f32,
        temperature: f32,
        #[serde(rename = "templow")]
        temperature_low: f32,
        wind_speed: f32,
        precipitation: u8,
        humidity: u8,
    }

    #[derive(Deserialize, Debug)]
    pub struct StateLightAttributes<'a> {
        min_color_temp_kelvin: u16,
        max_color_temp_kelvin: u16,
        min_mireds: u16,
        max_mireds: u16,
        supported_color_modes: Vec<ColorMode>,
        #[serde(borrow)]
        mode: Cow<'a, str>,
        #[serde(borrow)]
        dynamics: Cow<'a, str>,
        #[serde(borrow)]
        friendly_name: Cow<'a, str>,
        color_mode: Option<ColorMode>,
        brightness: Option<u8>,
        color_temp_kelvin: Option<u16>,
        color_temp: Option<u16>,
        xy_color: Option<(u8, u8)>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "snake_case")]
    pub enum ColorMode {
        ColorTemp,
        Xy,
    }
}
