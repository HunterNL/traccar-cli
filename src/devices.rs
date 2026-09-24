use serde::{Deserialize, Serialize};

use crate::{TracarrError, Traccar};

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

#[derive(Debug)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub position_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub hidden: Option<bool>,
    pub display_name: Option<String>,
    pub report_timeout_seconds: Option<u32>,
    pub predict_update_interval_seconds: Option<u32>,
}

impl Device {
    fn from_response(r: DeviceReponse) -> Self {
        Self {
            id: r.id,
            name: r.name,
            position_id: match r.position_id {
                0 => None,
                a => Some(a),
            },
        }
    }
}

impl Traccar {
    pub async fn list_devices(&self) -> Result<Vec<Device>, crate::TracarrError> {
        let response = self.prepare_request("/api/devices").send().await?;
        let out: Vec<DeviceReponse> = Self::get_json(response).await?;

        let out = out.into_iter().map(Device::from_response).collect();

        Ok(out)
    }

    pub async fn device_get(&self, device_id: u32) -> Result<Device, TracarrError> {
        let response = self
            .prepare_request("/api/devices")
            .query(&[("id", device_id)])
            .send()
            .await?;
        let out: Vec<DeviceReponse> = Self::get_json(response).await?;

        Ok(out
            .into_iter()
            .map(Device::from_response)
            .find(|device| device.id == device_id)
            .expect("did not return device"))
    }
}
