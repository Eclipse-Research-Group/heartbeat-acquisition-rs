use std::path::{Path, PathBuf};

pub mod csv;
pub mod hdf5;

pub trait Writer {
    fn new(path: PathBuf) -> anyhow::Result<Self> where Self: Sized;
    fn close(self) -> anyhow::Result<()>;
    async fn write_frame(&mut self, frame: &crate::serial::Frame) -> anyhow::Result<()>;
}