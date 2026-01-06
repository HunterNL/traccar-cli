use geo::Polygon;
use serde::Deserialize;

use crate::Traccar;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeoFenceResponse {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "wkt::deserialize_wkt")]
    pub area: Polygon,
}

impl Traccar {
    pub async fn geofences_all(&self) -> Vec<GeoFenceResponse> {
        let req = self.prepare_request("/api/geofences");
        let req = req.query(&[("all", "true")]);

        // let res: String = req.send().await.unwrap().text().await.unwrap();
        let res: Vec<GeoFenceResponse> = req.send().await.unwrap().json().await.unwrap();

        res
    }
}
