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

#[derive(Debug)]
pub struct Device {
    pub id: u32,
    pub name: String,
    pub position_id: Option<u32>,
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
}
