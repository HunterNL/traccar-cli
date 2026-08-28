use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{TracarrError, Traccar};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PositionResponse {
    pub id: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub fix_time: DateTime<Utc>,
    pub geofence_ids: Option<Vec<u32>>,
    pub device_id: u32,
    pub attributes: HashMap<String, serde_json::Value>,
}

impl PositionResponse {
    fn into_position(self) -> Position {
        Position {
            id: self.id,
            latitude: self.latitude,
            longitude: self.longitude,
            altitude: self.altitude,
            fix_time: self.fix_time,
            geofence_ids: self.geofence_ids.unwrap_or_default(),
            device_id: self.device_id,
            attributes: self.attributes,
        }
    }
}

#[derive(Default, Debug)]
pub struct Position {
    pub id: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub fix_time: DateTime<Utc>,
    pub geofence_ids: Vec<u32>,
    pub device_id: u32,
    attributes: HashMap<String, serde_json::Value>,
}

impl Position {
    pub fn battery_level(&self) -> Option<f64> {
        self.attributes
            .get("battery")
            .and_then(|json| json.as_f64())
    }
}

impl Traccar {
    pub async fn position_get(&self, position_id: u32) -> Result<Position, TracarrError> {
        let req = self.prepare_request("/api/positions");
        let req = req.query(&[("id", position_id)]);
        let response = req.send().await?;

        // let text = req
        //     .try_clone()
        //     .unwrap()
        //     .send()
        //     .await
        //     .unwrap()
        //     .text()
        //     .await
        //     .unwrap();

        let res: Vec<PositionResponse> = Self::get_json(response).await?;

        res.into_iter()
            .map(|a| a.into_position())
            .next()
            .ok_or(TracarrError::EmptyPositionResponse)
    }
}
