use std::{collections::HashMap, fs, path::PathBuf};

use geo::Point;
use serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize)]
pub struct ConfigFile {
    pub host: Option<String>,
    pub token: Option<String>,
    pub devices: Option<HashMap<u32, DeviceConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub display_name: Option<String>,
    pub report_timeout_seconds: Option<u32>,
    pub predict_update_interval_seconds: Option<u32>,
}
//     AppConfig {
//     devices: config_file.devices.unwrap_or_default(),
//     landmarks,
//     host: config_file.host,
//     token: config_file.token,
// }se();

impl AppConfig {
    pub fn from_config_file(file: &ConfigFile, landmarks: Vec<Landmark>) -> Self {
        Self {
            landmarks,
            host: file.host.as_ref().unwrap().to_owned(),
            token: file.token.as_ref().unwrap().to_owned(),
            devices: file.devices.as_ref().unwrap().to_owned(),
        }
    }

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

pub fn dir() -> PathBuf {
    dirs::config_local_dir().unwrap().join("traccar")
}

fn config_path() -> PathBuf {
    dir().join("config.json")
}

pub fn landmarks_read() -> Vec<Landmark> {
    let path = dir().join("landmarks.json");

    let landmarks = fs::read_to_string(path).unwrap();
    let config: Vec<LandmarkConfig> = serde_json::from_str(&landmarks).unwrap();
    config
        .into_iter()
        .map(|landmark| Landmark {
            name: landmark.name,
            position: Point::new(landmark.location.lng, landmark.location.lat),
        })
        .collect()
}
pub fn config_read() -> ConfigFile {
    let config_file: ConfigFile = {
        let file = fs::read_to_string(config_path()).unwrap();
        serde_json::from_str(&file).unwrap()
    };

    config_file
}

pub fn config_write(config: &ConfigFile) {
    let bytes = serde_json::to_vec_pretty(&config).unwrap();
    std::fs::write(config_path(), bytes).unwrap();
}
