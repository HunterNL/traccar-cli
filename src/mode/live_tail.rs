use std::collections::VecDeque;
use std::fmt;
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use tokio::time::{Sleep, sleep};
use tokio_tungstenite::connect_async_tls_with_config;

use futures::{Sink, SinkExt, StreamExt};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Message, http};
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;
use traccar_lib::{Device, GeoFenceResponse, PositionResponse};
use traccar_lib::{DeviceReponse, Position};
// use traccar_lib::{DeviceReponse, Position, PositionResponse};

use crate::Landmark;
use crate::config::{AppConfig, DeviceConfig};
use crate::mode::report_once::report_device;

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

/// Tail handles receiving data from a source like websockets and only emitting updates when the data is newer and complete
#[derive(Debug)]
struct Tail {
    device_filter: u32,
    last_sent_update: Option<u32>, //PositionId

    // devices: VecDeque<DeviceReponse>,
    positions: VecDeque<Position>,
}

fn select_most_recent_position<'a>(a: &'a Position, b: &'a Position) -> &'a Position {
    if a.fix_time > b.fix_time { a } else { b }
}

impl Tail {
    fn apply_update(&mut self, positions: &[Position]) -> bool {
        // dbg!(&self);
        positions
            .iter()
            .filter(|a| a.device_id == self.device_filter)
            .for_each(|position| {
                // self.positions.insert(index, value);
                self.positions.push_front(position.clone());
            });
        // .reduce(select_most_recent_position);

        // Clean up the queue, removing the oldest entry
        // There's a tiny chance this removes an entry that would've been the newest one if traccar sent things in a really weird order
        // but worst case this only causes an update to skip, order should be preserved
        self.positions.truncate(16);

        let most_recent_position = self.positions.iter().max_by_key(|p| p.fix_time);

        // Got empty history, no changes
        let most_recent_position = match most_recent_position {
            Some(a) => a,
            None => return false,
        };

        // Got a history now and we've never send an update, send one for sure
        if self.last_sent_update.is_none() {
            self.last_sent_update = Some(most_recent_position.id);
            return true;
        }

        let last_sent_update = self.last_sent_update.unwrap(); // Safe, since we would've returned before

        // We got a history and we've send an update before, compare to see if we've got a new newest
        let recent_update = self.positions.iter().find(|a| a.id == last_sent_update);

        match recent_update {
            Some(recent_update) => {
                if most_recent_position.fix_time > recent_update.fix_time {
                    self.last_sent_update = Some(most_recent_position.id);
                    true
                } else {
                    false
                }
            }
            None => {
                // oops, somehow the most recent postion is not to be found in the history?!
                self.last_sent_update = Some(most_recent_position.id);
                true
            }
        }
    }

    fn most_recent(&self) -> Option<&Position> {
        self.positions.iter().reduce(select_most_recent_position)
    }

    fn new(device_id: u32) -> Self {
        Self {
            device_filter: device_id,
            last_sent_update: None,
            positions: Default::default(),
        }
    }
}

async fn handle_message(
    msg: Result<Message, tokio_tungstenite::tungstenite::Error>,
    tail: &mut Tail, // mut write2: S,
    device: &Device,
    landmarks: &[Landmark],
    geofences: &[GeoFenceResponse],
    device_config: Option<DeviceConfig>,
)
where
// <S as futures::Sink<tokio_tungstenite::tungstenite::Message>>::Error: std::fmt::Debug,
{
    if let Ok(Message::Text(text)) = msg {
        let r: WebSocketResponse = serde_json::from_str(text.as_str()).unwrap();
        // dbg!(
        //     &r.positions
        //         .iter()
        //         .flatten()
        //         .map(|a| a.fix_time)
        //         .collect::<Vec<_>>()
        // );
        let positions: Vec<Position> = r
            .positions
            .into_iter()
            .flatten()
            .map(PositionResponse::into_position)
            .collect();
        let did_update = tail.apply_update(&positions);
        if did_update {
            let recent = tail.most_recent();

            if let Some(recent) = recent {
                let now = Utc::now();

                let report = report_device(
                    device,
                    recent,
                    &vec![],
                    landmarks,
                    device_config.as_ref(),
                    now,
                );

                println!("{}", report);
            }

            // dbg!(tail.most_recent().unwrap().fix_time);
        };
    } else if let Ok(Message::Pong(_)) = msg {
        // dbg!("Pong");
    } else {
        // dbg!("err", &msg);
    }
}

pub async fn tail_devices(
    config: AppConfig,
    cancel_token: CancellationToken,
    device_id: u32,
    landmarks: &[Landmark],
) {
    let client = traccar_lib::Traccar::new(config.host(), config.token()).expect("clinet");

    let devices = client.list_devices().await.unwrap();
    let device = devices.iter().find(|a| a.id == device_id).unwrap();
    let geofences = client.geofences_all().await.unwrap();
    let device_config = config.device_config(device_id).cloned();

    // let devices = client.list_devices().await;

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

    let mut tail = Tail::new(device_id);

    tokio::spawn(async move {
        loop {
            tokio::select! {
               _= token2.cancelled() => {
                    break;
                },
                _ = sleep(KEEPALIVE_INTERVAL) => {
                    // println!("Sending ping");
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
                    Some(a) => handle_message(a,&mut tail/*, &mut write2 */,&device,&landmarks,&geofences,device_config.clone()).await,
                }
            }

        }
    }

    // todo!()
}
