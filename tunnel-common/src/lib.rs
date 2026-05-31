// Tunnel Common Library
use serde;
pub struct Registry {}
pub trait Service {
    fn name(&self) -> String;
}
pub struct ServiceId(String);
impl From<String> for ServiceId {
    fn from(s: String) -> Self { ServiceId(s) }
}
