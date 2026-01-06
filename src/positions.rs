use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::Traccar;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PositionResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub fix_time: DateTime<Utc>,
    pub geofence_ids: Option<Vec<u32>>,
}
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub fix_time: DateTime<Utc>,
    pub geofence_ids: Vec<u32>,
}

impl Traccar {
    pub async fn position_get(&self, position_id: u32) -> Position {
        let req = self.prepare_request("/api/positions");
        let req = req.query(&[("id", position_id)]);

        let res: Vec<PositionResponse> = req.send().await.unwrap().json().await.unwrap();

        res.into_iter()
            .map(|a| Position {
                latitude: a.latitude,
                longitude: a.longitude,
                altitude: a.altitude,
                fix_time: a.fix_time,
                geofence_ids: a.geofence_ids.unwrap_or_default(),
            })
            .next()
            .unwrap()
    }
}
