use uuid::Uuid;
use std::collections::HashMap;

static METADATA_START: &str = "## BEGIN METADATA ##\n";
static METADATA_END: &str = "## END METADATA ##\n";

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