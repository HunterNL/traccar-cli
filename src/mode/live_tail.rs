use std::collections::VecDeque;
use std::time::Duration;

use chrono::Utc;
use serde::Deserialize;
use tokio::time::sleep;
use tokio_tungstenite::connect_async_tls_with_config;

use futures::{SinkExt, StreamExt};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Message, http};
use tokio_util::sync::CancellationToken;
use traccar_lib::{Device, PositionResponse, Reporter};
use traccar_lib::{DeviceReponse, Position};

use crate::config::AppConfig;
// use traccar_lib::{DeviceReponse, Position, PositionResponse};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Deserialize, Debug)]
struct WebSocketResponse {
    positions: Option<Vec<PositionResponse>>,
    _devices: Option<Vec<DeviceReponse>>,
    _events: Option<serde_json::Value>,
}

// impl WebSocketResponse {
//     /// Returns if this [`WebSocketResponse`] is actually a traccar keepalive message.
//     pub fn is_keepalive(&self) -> bool {
//         self.positions.is_none() && self._devices.is_none() && self._events.is_none()
//     }
// }

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
        for position in positions {
            if position.device_id == self.device_filter {
                self.positions.push_front(position.clone());
            }
        }

        // Clean up the queue, removing the oldest entry
        // There's a tiny chance this removes an entry that would've been the newest one if traccar sent things in a really weird order
        // but worst case this only causes an update to skip, order should be preserved
        self.positions.truncate(16);

        // Get the most recent position we have
        // If we don't have any we've got nothing to emit
        let Some(most_recent_position) = self.positions.iter().max_by_key(|p| p.fix_time) else {
            return false;
        };

        // Got a history now and we've never send an update, send one for sure
        if self.last_sent_update.is_none() {
            self.last_sent_update = Some(most_recent_position.id);
            return true;
        }

        let last_sent_update = self.last_sent_update.unwrap(); // Safe, since we would've returned before

        // We got a history and we've send an update before, compare to see if we've got a new newest
        let recent_update = self.positions.iter().find(|a| a.id == last_sent_update);

        if let Some(recent_update) = recent_update {
            if most_recent_position.fix_time > recent_update.fix_time {
                self.last_sent_update = Some(most_recent_position.id);
                true
            } else {
                false
            }
        } else {
            // oops, somehow the most recent postion is not to be found in the history?!
            self.last_sent_update = Some(most_recent_position.id);
            true
        }
    }

    fn most_recent(&self) -> Option<&Position> {
        self.positions.iter().reduce(select_most_recent_position)
    }

    fn new(device_id: u32) -> Self {
        Self {
            device_filter: device_id,
            last_sent_update: None,
            positions: VecDeque::default(),
        }
    }
}

fn handle_message(
    msg: Result<Message, tokio_tungstenite::tungstenite::Error>,
    tail: &mut Tail, // mut write2: S,
    device: &Device,
    reporter: &Reporter,
)
where
// <S as futures::Sink<tokio_tungstenite::tungstenite::Message>>::Error: std::fmt::Debug,
{
    if let Ok(Message::Text(text)) = msg {
        let r: WebSocketResponse = serde_json::from_str(text.as_str()).unwrap();
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

                // dbg!(reporter);
                let report = reporter.report_device(device, recent, now);

                println!("{report}");
            }

            // dbg!(tail.most_recent().unwrap().fix_time);
        }
    } else if let Ok(Message::Pong(_)) = msg {
        // dbg!("Pong");
    } else {
        // dbg!("err", &msg);
    }
}

pub async fn tail_devices(
    config: AppConfig,
    reporter: Reporter,
    device_id: u32,
    cancel_token: CancellationToken,
) {
    let client = traccar_lib::Traccar::new(config.host(), config.token()).expect("clinet");

    let devices = client.list_devices().await.unwrap();
    let device = devices.iter().find(|a| a.id == device_id).unwrap();

    let url: http::Uri =
        (String::new() + config.host() + "/api/socket" + "?token=" + config.token())
            .parse()
            .unwrap();
    let mut p = url.into_parts();
    p.scheme = Some("wss".try_into().unwrap());

    let url: http::Uri = p.try_into().unwrap();

    // let url = String::new() + "ws://" + config.host() + "/api/websocket";
    // println!("{url}");
    let request = url.into_client_request().unwrap();
    let a = native_tls::TlsConnector::new().unwrap();
    let a = tokio_tungstenite::Connector::NativeTls(a);
    let res = connect_async_tls_with_config(request, None, false, Some(a)).await;
    if res.is_err() {
        println!("{res:?}");
    }

    let (streams, _) = res.unwrap();
    let (mut write_stream, mut read_stream) = streams.split();

    // TODO maybe redundant?
    let token_keepalive = cancel_token.clone();

    let mut tail = Tail::new(device_id);

    // Spawn another task purely to send keepalives
    tokio::spawn(async move {
        loop {
            tokio::select! {
               ()= token_keepalive.cancelled() => {
                    break;
                },
                () = sleep(KEEPALIVE_INTERVAL) => {
                    // println!("Sending ping");
                    write_stream.send(Message::Ping(Vec::new().into())).await.unwrap();
                }

            }
        }
    });

    // Actual worker, handles receiving messages
    loop {
        tokio::select! {
            () = cancel_token.cancelled() => {
                break
            },
            msg = read_stream.next() => {
                match msg {
                    None => break, // Empty message means the stream ended, break the loop
                    Some(a) => handle_message(a,&mut tail/*, &mut write2 */,device,&reporter),
                }
            }

        }
    }

    // todo!()
}
