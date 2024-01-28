#[derive(Debug)]
pub struct DataPointFlags {
    has_gps_fix: bool,
    is_clipping: bool
}

impl ToString for DataPointFlags {
    fn to_string(&self) -> String {
        let mut result: String = String::new();
        if self.has_gps_fix {
            result.push('G');
        }

        if self.is_clipping {
            result.push('O');
        }

        return result;
    }
}


impl DataPointFlags {
    fn new() -> DataPointFlags {
        DataPointFlags {
            has_gps_fix: false,
            is_clipping: false
        }
    }

    fn parse(line: &str) -> DataPointFlags {
        let mut flags = DataPointFlags::new();

        if line.contains('G') {
            flags.has_gps_fix = true;
        }

        if line.contains('O') {
            flags.is_clipping = true;
        }

        return flags;
    }
}

#[derive(Debug)]
pub struct DataPoint {
    timestamp: u32,
    sample_rate: f32,
    flags: DataPointFlags,
    latitude: f32,
    longitude: f32,
    elevation: f32,
    fix: u16,
    data_string: Option<String>
}

impl DataPoint {

    pub fn has_gps_fix(&self) -> bool {
        self.flags.has_gps_fix
    }

    pub fn is_clipping(&self) -> bool {
        self.flags.is_clipping
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn latitude(&self) -> f32 {
        self.latitude
    }

    pub fn longitude(&self) -> f32 {
        self.longitude
    }

    pub fn elevation(&self) -> f32 {
        self.elevation
    }

    pub fn fix(&self) -> u16 {
        self.fix
    }

    pub fn parse(line: &str) -> Result<DataPoint, String> {
        let parts: Vec<&str> = line.split(',').collect();
        let timestamp = match parts[0].parse::<u32>().unwrap() {
            timestamp => timestamp,
            _ => return Err("Failed to parse timestamp".to_string())
        };

        let flags = match DataPointFlags::parse(parts[1]) {
            flags => flags,
            _ => return Err("Failed to parse flags".to_string())
        };

        let sample_rate = match parts[2].parse::<f32>().unwrap() {
            sample_rate => sample_rate,
            _ => return Err("Failed to parse sample rate".to_string())
        };

        let latitude = match parts[3].parse::<f32>().unwrap() {
            latitude => latitude,
            _ => return Err("Failed to parse latitude".to_string())
        };
        

        let longitude = match parts[4].parse::<f32>().unwrap() {
            longitude => longitude,
            _ => return Err("Failed to parse longitude".to_string())
        };


        let elevation = match parts[5].parse::<f32>().unwrap() {
            elevation => elevation,
            _ => return Err("Failed to parse elevation".to_string())
        };

        let fix = match parts[6].parse::<u16>().unwrap() {
            fix => fix,
            _ => return Err("Failed to parse fix".to_string())
        };

        let speed = match parts[7].parse::<f32>().unwrap() {
            speed => speed,
            _ => return Err("Failed to parse speed".to_string())
        };
        
        let angle = match parts[8].parse::<f32>().unwrap() {
            angle => angle,
            _ => return Err("Failed to parse angle".to_string())
        };


        let data_point = DataPoint {
            timestamp: timestamp,
            sample_rate: sample_rate,
            flags: flags,
            latitude: latitude,
            longitude: longitude,
            elevation: elevation,
            fix: fix,
            data_string: Some("not implemented".to_string())
        };

        return Ok(data_point);
    }

    pub fn satellite_count(&self) -> u16 {
        self.fix
    }
}

impl ToString for DataPoint {
    fn to_string(&self) -> String {
        let string: String = format!("{},{}", self.timestamp, self.flags.to_string());
        return string;
    }
}