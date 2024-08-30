use std::path::{Path, PathBuf};

use chrono::Utc;

pub mod csv;
pub mod hdf5;

pub trait Writer<C> where C: Clone {
    fn new(config: C) -> anyhow::Result<Self> where Self: Sized;
    fn close(self) -> anyhow::Result<()>;
    async fn write_frame(&mut self, frame_when: chrono::DateTime<Utc>, frame: &crate::serial::Frame) -> anyhow::Result<()>;
    async fn write_comment(&mut self, comment: &str) -> anyhow::Result<()>;
}