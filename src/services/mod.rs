pub mod local;

#[derive(Debug, Clone)]
pub enum ServiceMessage {
    NewFrame(crate::serial::Frame),
    Shutdown
}