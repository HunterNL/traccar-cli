use geo::Polygon;
use serde::Deserialize;

use crate::Traccar;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GeoFenceResponse {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "wkt::deserialize_wkt")]
    pub area: Polygon,
}

impl Traccar {
    pub async fn geofences_all(&self) -> Result<Vec<GeoFenceResponse>, crate::TracarrError> {
        let request = self.prepare_request("/api/geofences");
        let request = request.query(&[("all", "true")]);
        let response = request.send().await?;

        let res: Vec<GeoFenceResponse> = Self::get_json(response).await?;

        Ok(res)
    }
}
