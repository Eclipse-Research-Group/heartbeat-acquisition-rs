use uuid::Uuid;
use std::collections::HashMap;

static METADATA_START: &str = "## BEGIN METADATA ##";
static METADATA_END: &str = "## END METADATA ##";

pub struct CaptureFileMetadata {
    capture_id: Uuid,
    node_id: String,
    extras: HashMap<String, String>
}

impl CaptureFileMetadata {
    pub fn new(capture_id: Uuid, node_id: String) -> CaptureFileMetadata {
        CaptureFileMetadata {
            capture_id: capture_id,
            node_id: node_id,
            extras: HashMap::new()
        }
    }

    pub fn set(&mut self, key: String, value: String) {
        self.extras.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.extras.get(key).map(|s| s.as_str())
    }

    pub fn parse(text: &str) -> CaptureFileMetadata {
        return CaptureFileMetadata::new(Uuid::new_v4(), String::from(""));
    }
}

impl ToString for CaptureFileMetadata {
    fn to_string(&self) -> String {
        let string: String = format!("Capture ID: {}\nNode ID: {}\n", self.capture_id, self.node_id);
        return string;
    }
}