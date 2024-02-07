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

fn parse(line: &str) -> Result<DataPointFlags, String> {
        let mut flags = DataPointFlags::new();

        if line.contains('G') {
            flags.has_gps_fix = true;
        }

        if line.contains('O') {
            flags.is_clipping = true;
        }

        return Ok(flags);
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
    data: Vec<f64>
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

    pub fn satellites(&self) -> u16 {
        self.fix
    }

    pub fn data(&self) -> &Vec<f64> {
        &self.data
    }

    pub fn parse(line: &str) -> Result<DataPoint, String> {
        let mut iter = line.split(',');

        let part = iter.next().ok_or("Missing timestamp")?;
        let timestamp = match part.parse::<i64>() {
            Ok(timestamp) => timestamp,
            _ => return Err("Failed to parse timestamp".to_string())
        };

        let part = iter.next().ok_or("Missing flags")?;
        let flags = match DataPointFlags::parse(part) {
            Ok(flags) => flags,
            _ => return Err("Failed to parse flags".to_string())
        };

        let part = iter.next().ok_or("Missing sample rate")?;
        let sample_rate = match part.parse::<f32>() {
            Ok(sample_rate) => sample_rate,
            _ => return Err("Failed to parse sample rate".to_string())
        };

        let part = iter.next().ok_or("Missing latitude")?;
        let latitude = match part.parse::<f32>() {
            Ok(latitude) => latitude,
            _ => return Err("Failed to parse latitude".to_string())
        };
        
        let part = iter.next().ok_or("Missing longitude")?;
        let longitude = match part.parse::<f32>() {
            Ok(longitude) => longitude,
            _ => return Err("Failed to parse longitude".to_string())
        };

        let part = iter.next().ok_or("Missing elevation")?;
        let elevation = match part.parse::<f32>() {
            Ok(elevation) => elevation,
            _ => return Err("Failed to parse elevation".to_string())
        };

        let part = iter.next().ok_or("Missing fix")?;
        let fix = match part.parse::<u16>() {
            Ok(fix) => fix,
            _ => return Err("Failed to parse fix".to_string())
        };

        let part = iter.next().ok_or("Missing speed")?;
        let speed = match part.parse::<f32>() {
            Ok(speed) => speed,
            _ => return Err("Failed to parse speed".to_string())
        };
        
        let part = iter.next().ok_or("Missing angle")?;
        let angle = match part.parse::<f32>() {
            Ok(angle) => angle,
            _ => return Err("Failed to parse angle".to_string())
        };

        let part = iter.next().ok_or("Missing data count")?;
        let data_count: usize = match part.parse::<u16>() {
            Ok(data_count) => data_count as usize,
            _ => return Err("Failed to parse data count".to_string())
        };

        log::info!("Data count: {}", data_count);

        let mut data = Vec::<f64>::new();
        let mut sum = 0u64;
        for i in 10..10usize + data_count {
            let part = iter.next().ok_or("Missing data")?;
            let value = match part.parse::<i64>() {
                Ok(value) => value,
                _ => return Err("Failed to parse data".to_string())
            };

            sum += value as u64;
            // let value = part.parse::<i64>().unwrap();
            data.push((value - 512) as f64 / 512.0); 
        }

        let checksum = atoi::atoi::<u64>(iter.next().ok_or("Missing checksum")?.as_bytes()).unwrap();

        if checksum != sum {
            return Err("Checksum failed".to_string());
        }

        let data_point = DataPoint {
            timestamp: timestamp as u32,
            sample_rate: sample_rate,
            flags: flags,
            latitude: latitude,
            longitude: longitude,
            elevation: elevation,
            fix: fix,
            data: data
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