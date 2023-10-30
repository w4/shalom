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
}
