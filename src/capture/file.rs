use crate::capture::data::{DataPointFlags, DataPoint};
use uuid::Uuid;
use std::path::Path;
use std::{collections::HashMap, fs::File};
use std::io::Write;
use chrono::{DateTime, Utc};

static METADATA_START: &str = "## BEGIN METADATA ##\n";
static METADATA_END: &str = "## END METADATA ##\n";

#[derive(Clone)]
pub struct CaptureFileMetadata {
    capture_id: Uuid,
    sample_rate: f32,
    extras: HashMap<String, String>
}

impl CaptureFileMetadata {
    pub fn new(capture_id: Uuid, sample_rate: f32) -> CaptureFileMetadata {
        CaptureFileMetadata {
            capture_id: capture_id,
            sample_rate: sample_rate,
            extras: HashMap::new()
        }
    }

    pub fn set (&mut self, key: &str, value: &str) {
        self.extras.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.extras.get(key).map(|s| s.as_str())
    }

    pub fn parse(text: &str) -> CaptureFileMetadata {
        let parts = text.split("\n").collect::<Vec<&str>>();
        

        return CaptureFileMetadata::new(Uuid::new_v4(), 20000.0);
    }

    pub fn capture_id(&self) -> Uuid {
        self.capture_id
    }
}

impl ToString for CaptureFileMetadata {
    fn to_string(&self) -> String {
        let mut string = String::new();
        string.push_str(METADATA_START);
        string.push_str(format!("# CAPTURE_ID\t\t{}\n# SAMPLE_RATE\t\t{}\n", self.capture_id, self.sample_rate).as_str());

        // Write additional metadata
        for (key, value) in &self.extras {
            string.push_str(format!("# {}\t\t{}\n", key, value).as_str());
        }

        string.push_str(METADATA_END);
        return string;
    }
}

pub struct CaptureFileWriter {
    dir: Box<Path>,
    created: DateTime<Utc>,
    metadata: CaptureFileMetadata,
    file: File,
    filename: String,
    lines_written: usize
}

impl CaptureFileWriter {

    pub fn new(dir: &Path, metadata: &mut CaptureFileMetadata) -> Result<CaptureFileWriter, std::io::Error> {
        let created = Utc::now();
        metadata.set("CREATED", created.to_rfc3339().as_str());
        let node_id = metadata.get("NODE_ID").unwrap_or("UNKNOWN");
        std::fs::create_dir_all(dir)?;
        let filename = format!("{}_{}_{}.csv", node_id, created.format("%Y%m%d_%H%M%S"), metadata.capture_id.to_string()[..8].to_string());
        let file = File::create(dir.join(&filename))?;
        log::info!("Created file: {}", dir.join(&filename).as_mut_os_str().to_str().unwrap());
        Ok(CaptureFileWriter {
            dir: dir.into(),
            created: created,
            metadata: metadata.clone(),
            file: file,
            filename: filename,
            lines_written: 0
        })
    }

    pub fn init(&mut self) {
        let string = self.metadata.to_string();
        self.file.write_all(string.as_bytes()).unwrap();
    }

    pub fn write_data(&mut self, data_point: DataPoint) {
        // Write data
        let string = data_point.to_string();
        self.file.write_all(string.as_bytes()).unwrap();
    }

    pub fn write_line(&mut self, line: &str) {
        self.file.write_all(line.as_bytes()).unwrap();
        self.file.flush().unwrap();
        self.lines_written += 1;
    }

    pub fn filename(&self) -> String {
        self.filename.clone()
    }

    pub fn comment(&mut self, comment: &str) {
        self.write_line(format!("# {}", comment).as_str());
    }

    pub fn lines_written(&self) -> usize {
        self.lines_written
    }

}