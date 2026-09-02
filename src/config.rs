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

/// The configuration as 'required' by most of the app
#[derive(Debug, Clone)]
pub struct AppConfig {
    landmarks: Vec<Landmark>,
    host: String,
    token: String,
    devices: HashMap<u32, DeviceConfig>,
}

/// The raw configuration file, for serialization
#[derive(Deserialize, Serialize)]
pub struct ConfigFile {
    pub host: Option<String>,
    pub token: Option<String>,
    pub devices: Option<HashMap<u32, DeviceConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub hidden: Option<bool>,
    pub display_name: Option<String>,
    pub report_timeout_seconds: Option<u32>,
    pub predict_update_interval_seconds: Option<u32>,
}
#[derive(Debug, Clone)]
pub struct ConfigBase {
    dir: PathBuf,
    landmarks: PathBuf,
    config_file: PathBuf,
}

impl ConfigBase {
    pub fn new(path: PathBuf) -> Self {
        if !path.is_dir() {
            panic!("Path passed to ConfigBase::new is not a directory");
        }
        let path = path.canonicalize().expect("to canonicalize path");
        Self {
            landmarks: path.join("landmarks.json"),
            config_file: path.join("config.json"),
            dir: path,
        }
    }

    // pub fn dir(&self) -> &Path {
    //     &self.dir
    // }

    pub fn read_config_file(&self) -> Option<ConfigFile> {
        fs::read_to_string(&self.config_file)
            .ok()
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    pub fn write_config_file(&self, config: &ConfigFile) {
        let bytes = serde_json::to_vec_pretty(config).unwrap();
        std::fs::write(&self.config_file, bytes).unwrap();
    }

    pub fn read_landmark_file(&self) -> Option<Vec<Landmark>> {
        match fs::read_to_string(&self.landmarks) {
            Ok(landmark_str) => {
                let config: Vec<LandmarkConfig> = serde_json::from_str(&landmark_str).unwrap();
                Some(
                    config
                        .into_iter()
                        .map(|landmark| Landmark {
                            name: landmark.name,
                            position: Point::new(landmark.location.lng, landmark.location.lat),
                        })
                        .collect(),
                )
            }
            Err(_) => None,
        }
    }
}

impl Default for ConfigBase {
    fn default() -> Self {
        Self::new(dirs::config_local_dir().unwrap().join("traccar"))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppConfigError {
    #[error("Host is missing")]
    MissingHost,
    #[error("Token is missing")]
    MissingToken,
}

impl AppConfig {
    pub fn from_config_file(
        file: &ConfigFile,
        landmarks: Vec<Landmark>,
    ) -> Result<Self, AppConfigError> {
        let host = file
            .host
            .as_ref()
            .ok_or(AppConfigError::MissingHost)?
            .clone();
        let token = file
            .token
            .as_ref()
            .ok_or(AppConfigError::MissingToken)?
            .clone();

        let devices = file.devices.clone().unwrap_or_default();

        Ok(Self {
            landmarks,
            host,
            token,
            devices,
        })
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

// pub fn config_write(config: &ConfigFile) {}
