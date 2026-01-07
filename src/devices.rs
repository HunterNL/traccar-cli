use serde::Deserialize;

use crate::Traccar;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceReponse {
    pub id: u32,
    pub name: String,
    // pub status: String,
    // last_update: DateTime<Utc>,
    pub position_id: u32,
    // geofences: Vec<u32>,
}

impl Traccar {
    pub async fn list_devices(&self) -> Vec<DeviceReponse> {
        let response = self.prepare_request("/api/devices").send().await.unwrap();

        response.json().await.unwrap()
    }
}
