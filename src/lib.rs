use reqwest::Response;
use reqwest::{IntoUrl, Url};

mod devices;
mod geofences;
mod positions;
mod session;

pub use devices::Device;
pub use geofences::GeoFenceResponse;
pub use positions::Position;
pub use positions::PositionResponse;
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
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Other")]
    Other,
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
        } else {
            Err(TracarrError::Other)
        }
    }
}
