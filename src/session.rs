use serde_json::Value;

use crate::Traccar;

// pub struct SessionResponse {
//     id: u32,
//     name: String,
//     email: String,
// }

impl Traccar {
    pub async fn session_get(&self) -> Option<Value> {
        let path = "/api/session";
        let path = self.host.clone().join(path).unwrap();

        let req = self.http_client.get(path);
        let req = req.query(&[("token", self.token.clone())]);

        match req.send().await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        }
    }
}
