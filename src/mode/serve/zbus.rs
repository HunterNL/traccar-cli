use zbus::interface;

use super::LocationService;
pub use zbus::connection::Builder;
pub use zbus::names::BusName;

#[interface(name = "life.vern.traccar")]
impl LocationService {
    // Can be `async` as well.
    pub(crate) fn Get(&mut self, id: u32) -> String {
        match self.location.get_by_id(id) {
            Some((_, report)) => report.position.to_string(),
            None => "Id not found".to_string(),
        }
    }
}
