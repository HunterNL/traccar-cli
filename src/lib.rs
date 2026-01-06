use reqwest::{IntoUrl, Url};

mod devices;
mod geofences;
mod positions;

pub use devices::DeviceReponse;
pub use geofences::GeoFenceResponse;
pub use positions::Position;

pub struct Traccar {
    token: String,
    http_client: reqwest::Client,
    host: Url,
}

// #[derive(Deserialize, Debug)]
// pub struct DeviceId(u32);

// #[derive(Deserialize, Debug)]
// pub struct Device {
//     id: DeviceId,
// }
impl Traccar {
    pub fn new(host: impl IntoUrl, token: impl Into<String>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            token: token.into(),
            host: host.into_url().unwrap(),
        }
    }

    fn prepare_request(&self, path: &str) -> reqwest::RequestBuilder {
        let path = self.host.clone().join(path).unwrap();
        self.http_client.get(path).bearer_auth(self.token.clone())
    }
}
