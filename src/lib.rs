use reqwest::Response;
use reqwest::{IntoUrl, Url};

// mod config;
mod devices;
mod geofences;
mod positions;
mod report;
mod reporter;
mod session;

pub use devices::Device;
pub use devices::DeviceConfig;
pub use devices::DeviceReponse;
pub use geofences::GeoFenceResponse;
pub use positions::Position;
pub use positions::PositionResponse;
pub use report::Report;
pub use reporter::Landmark;
pub use reporter::Reporter;
use serde::de::DeserializeOwned;

pub struct Traccar {
    token: String,
    http_client: reqwest::Client,
    host: Url,
}

#[derive(thiserror::Error, Debug)]
pub enum TracarrError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Json parsing error: {0}")]
    DecodingError(#[from] serde_json::Error),
    #[error("Empty position response")]
    EmptyPositionResponse,
    #[error("Unauthorized, token likely expired")]
    Unauthorized,
    #[error("HTTP transport error, {0} {1}")]
    HttpError(u16, &'static str),
    // #[error("Other")]
    // Other,
}

// #[derive(Deserialize, Debug)]
// pub struct DeviceId(u32);

// #[derive(Deserialize, Debug)]
// pub struct Device {
//     id: DeviceId,
// }

impl Traccar {
    pub fn new(host: impl IntoUrl, token: impl Into<String>) -> Result<Self, TracarrError> {
        let host = host.into_url()?;
        Ok(Self {
            http_client: reqwest::Client::new(),
            token: token.into(),
            host,
        })
    }

    fn prepare_request(&self, path: &str) -> reqwest::RequestBuilder {
        let path = self.host.clone().join(path).unwrap();
        self.http_client.get(path).bearer_auth(self.token.clone())
    }

    async fn get_json<T: DeserializeOwned>(response: Response) -> Result<T, TracarrError> {
        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else if status.as_u16() == 401 {
            Err(TracarrError::Unauthorized)
            // So turns out traccar is being a potato, it sends 401 when your token is missing
            // yet a bare 400 if your token is expired
            // making this kinda useless
        } else {
            Err(TracarrError::HttpError(
                status.as_u16(),
                status.canonical_reason().unwrap_or("unknown reason"),
            ))
        }
    }

    pub async fn get_device_position(
        &self,
        device_id: u32,
    ) -> Result<(Device, Position), TracarrError> {
        let device = self.device_get(device_id).await?;
        let position = self
            .position_get(
                device
                    .position_id
                    .expect("edge case, fresh device doesn't have a position"),
            )
            .await?;
        Ok((device, position))
    }
}
