use crate::Traccar;

pub struct SessionResponse {
    id: u32,
    name: String,
    email: String
}

impl Traccar {
    async fn session_get(&self) -> SessionResponse
}
