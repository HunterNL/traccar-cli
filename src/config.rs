use std::fs;

use geo::Point;
use serde::Deserialize;

use crate::Landmark;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LandmarkConfig {
    name: String,
    location: LandmarkConfigLocation,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LandmarkConfigLocation {
    lat: f64,
    lng: f64,
}

pub struct AppConfig {
    landmarks: Vec<Landmark>,
    host: String,
    token: String,
}

#[derive(Deserialize)]
pub struct ConfigFile {
    host: String,
    token: String,
}

impl AppConfig {
    pub fn landmarks(&self) -> &[Landmark] {
        self.landmarks.as_slice()
    }
    pub fn host(&self) -> &str {
        self.host.as_str()
    }
    pub fn token(&self) -> &str {
        self.token.as_str()
    }
}

pub fn config_get() -> AppConfig {
    let base_path = dirs::config_local_dir().unwrap().join("traccar");

    let landmarks: Vec<Landmark> = {
        let path = base_path.join("landmarks.json");

        let landmarks = fs::read_to_string(path).unwrap();
        let config: Vec<LandmarkConfig> = serde_json::from_str(&landmarks).unwrap();
        config
            .into_iter()
            .map(|landmark| Landmark {
                name: landmark.name,
                position: Point::new(landmark.location.lng, landmark.location.lat),
            })
            .collect()
    };

    let config_file: ConfigFile = {
        let path = base_path.join("config.json");
        let file = fs::read_to_string(path).unwrap();
        serde_json::from_str(&file).unwrap()
    };

    AppConfig {
        landmarks,
        host: config_file.host,
        token: config_file.token,
    }
}
