use std::{fs::File, io::Write, time::Duration};

use iced::futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::config::HomeAssistantConfig;

type SocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub enum ClientMessage {
    // Ready,
}

pub async fn create(config: HomeAssistantConfig) -> mpsc::Receiver<ClientMessage> {
    let (_send, recv) = mpsc::channel(10);

    let uri = format!("ws://{}/api/websocket", config.uri);
    let (mut connection, _response) = tokio_tungstenite::connect_async(&uri).await.unwrap();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                Some(message) = connection.next() => {
                    let message = message.unwrap();
                    eprintln!("recv: {message:?}");

                    #[allow(clippy::match_same_arms)]
                    match message {
                        Message::Pong(ts) => {
                            let ts = i128::from_be_bytes(ts.try_into().unwrap());
                            let ts = OffsetDateTime::from_unix_timestamp_nanos(ts).unwrap();

                            eprintln!("rtt: {}", OffsetDateTime::now_utc() - ts);
                        }
                        Message::Text(payload) => {
                            handle_hass_response(&config, serde_json::from_str(&payload).unwrap(), &mut connection).await;
                        }
                        Message::Close(_) => {
                            // eprintln!("Reconnecting...");
                            // connection = tokio_tungstenite::connect_async(&uri).await.unwrap().0;
                        }
                        _ => {}
                    }
                }
                _ = interval.tick() => {
                    connection.send(Message::Ping(OffsetDateTime::now_utc().unix_timestamp_nanos().to_be_bytes().to_vec())).await.unwrap();
                }
            }
        }
    });

    recv
}

async fn handle_hass_response(
    config: &HomeAssistantConfig,
    v: HassResponse,
    socket: &mut SocketStream,
) {
    #[allow(clippy::match_same_arms)]
    match v {
        HassResponse::AuthRequired => {
            socket
                .send(
                    HassRequest::Auth {
                        access_token: config.token.clone(),
                    }
                    .to_request(),
                )
                .await
                .unwrap();
        }
        HassResponse::AuthOk => {
            // Lists: [aliases, area_id, name, picture]
            // socket
            //     .send(HassRequest::AreaRegistry { id: 3 }.to_request())
            //     .await
            //     .unwrap();

            // Lists: [area_id, entity_id]
            // socket.send(HassRequest::EntityRegistry { id: 3 }.to_request())
            //     .await
            //     .unwrap();

            // Lists: versions, area id, manufacturer, etc
            // socket.send(HassRequest::DeviceRegistry { id: 3 }.to_request())
            //     .await
            //     .unwrap();
        }
        HassResponse::AuthInvalid => {}
        HassResponse::Result(value) => {
            File::create("test")
                .unwrap()
                .write_all(&serde_json::to_vec(&value).unwrap())
                .unwrap();
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "result")]
#[allow(clippy::enum_variant_names)]
enum HassResponse {
    AuthRequired,
    AuthOk,
    AuthInvalid,
    Result(serde_json::Value),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum HassRequest {
    Auth {
        access_token: String,
    },
    GetStates {
        id: u32,
    },
    #[serde(rename = "config/area_registry/list")]
    AreaRegistry {
        id: u32,
    },
    #[serde(rename = "config/entity_registry/list")]
    EntityRegistry {
        id: u32,
    },
    #[serde(rename = "config/device_registry/list")]
    DeviceRegistry {
        id: u32,
    },
}

impl HassRequest {
    pub fn to_request(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}
