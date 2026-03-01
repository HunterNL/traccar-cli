use std::{collections::HashMap, fs};

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

#[derive(Debug, Clone)]
pub struct AppConfig {
    landmarks: Vec<Landmark>,
    host: String,
    token: String,
    devices: HashMap<u32, DeviceConfig>,
}

#[derive(Deserialize)]
pub struct ConfigFile {
    host: String,
    token: String,
    devices: Option<HashMap<u32, DeviceConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    pub display_name: Option<String>,
    pub report_timeout_seconds: Option<u32>,
    pub predict_update_interval_seconds: Option<u32>,
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
    pub fn device_config(&self, id: u32) -> Option<&DeviceConfig> {
        self.devices.get(&id)
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
        devices: config_file.devices.unwrap_or_default(),
        landmarks,
        host: config_file.host,
        token: config_file.token,
    }
}
