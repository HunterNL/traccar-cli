use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use tokio::time::{Sleep, sleep};
use tokio_tungstenite::connect_async_tls_with_config;

use futures::{Sink, SinkExt, StreamExt};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Message, http};
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;
use traccar_lib::{DeviceReponse, Position, PositionResponse};

use crate::config::AppConfig;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Deserialize, Debug)]
struct WebSocketResponse {
    positions: Option<Vec<PositionResponse>>,
    devices: Option<Vec<DeviceReponse>>,
    events: Option<serde_json::Value>,
}

impl WebSocketResponse {
    /// Returns if this [`WebSocketResponse`] is actually a traccar keepalive message.
    pub fn is_keepalive(&self) -> bool {
        self.positions.is_none() && self.devices.is_none() && self.events.is_none()
    }
}

struct Tail<'a, 'b> {
    device: &'a DeviceReponse,
    last_position: Option<&'b Position>,
}

async fn handle_message(
    msg: Result<Message, tokio_tungstenite::tungstenite::Error>,
    // mut write2: S,
)
where
// <S as futures::Sink<tokio_tungstenite::tungstenite::Message>>::Error: std::fmt::Debug,
{
    if let Ok(Message::Text(text)) = msg {
        let r: WebSocketResponse = serde_json::from_str(text.as_str()).unwrap();
        // if r.is_keepalibe() {
        //     write2.send(Message::Ping(Vec::new().into())).await.unwrap()
        // }
        dbg!(r);
    } else {
        dbg!("err", &msg);
    }
}

pub async fn tail_devices(config: AppConfig, cancel_token: CancellationToken) {
    let client = traccar_lib::Traccar::new(config.host(), config.token());
    let devices = client.list_devices().await;

    let url: http::Uri =
        (String::new() + config.host() + "/api/socket" + "?token=" + config.token())
            .parse()
            .unwrap();
    let mut p = url.into_parts();
    p.scheme = Some("wss".try_into().unwrap());

    let url: http::Uri = p.try_into().unwrap();

    // let url = String::new() + "ws://" + config.host() + "/api/websocket";
    println!("{url}");
    let request = url.into_client_request().unwrap();
    let a = native_tls::TlsConnector::new().unwrap();
    let a = tokio_tungstenite::Connector::NativeTls(a);
    let res = connect_async_tls_with_config(request, None, false, Some(a)).await;
    if res.is_err() {
        println!("{res:?}")
    }

    let (streams, response) = res.unwrap();
    let (mut write2, mut read2) = streams.split();
    let token2 = cancel_token.clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
               _= token2.cancelled() => {
                    break;
                },
                _ = sleep(KEEPALIVE_INTERVAL) => {
                    println!("Sending ping");
                    write2.send(Message::Ping(Vec::new().into())).await.unwrap();
                }

            }
        }
    });

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                break
            },
            msg = read2.next() => {
                match msg {
                    None => break,
                    Some(a) => handle_message(a/*, &mut write2 */).await,
                }
            }

        }
    }

    // todo!()
}
