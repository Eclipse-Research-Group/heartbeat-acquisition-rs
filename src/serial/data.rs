use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    has_gps_fix: bool,
    is_clipping: bool,
}

impl FrameMetadata {

    pub fn parse(line: &str) -> anyhow::Result<FrameMetadata> {
        return Ok(FrameMetadata {
            has_gps_fix: line.contains('G'),
            is_clipping: line.contains('O'),
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct Frame {
    timestamp: Option<i64>,
    sample_rate: f32,
    metadata: FrameMetadata,
    latitude: f32,
    longitude: f32,
    elevation: f32,
    speed: f32,
    angle: f32,
    fix: u16,
    data: Vec<i16>,
}

impl Frame {

    pub fn parse(line: &str) -> anyhow::Result<Frame> {
        let line = if line.starts_with('$') {
            line.chars().skip(1).collect::<String>()
        } else {
            line.to_string()
        };

        let mut iter = line.split(',');

        let part = iter.next().ok_or(anyhow::anyhow!("Missing timestamp"))?;
        let timestamp = match part.parse::<i64>() {
            Ok(timestamp) => Some(timestamp),
            _ => None,
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing flags"))?;
        let metadata = FrameMetadata::parse(part)?;

        let part = iter.next().ok_or(anyhow::anyhow!("Missing sample rate"))?;
        let sample_rate = match part.parse::<f32>() {
            Ok(sample_rate) => sample_rate,
            _ => return Err(anyhow::anyhow!("Failed to parse sample rate")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing latitude"))?;
        let latitude = match part.parse::<f32>() {
            Ok(latitude) => latitude,
            _ => return Err(anyhow::anyhow!("Failed to parse latitude")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing longitude"))?;
        let longitude = match part.parse::<f32>() {
            Ok(longitude) => longitude,
            _ => return Err(anyhow::anyhow!("Failed to parse longitude")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing elevation"))?;
        let elevation = match part.parse::<f32>() {
            Ok(elevation) => elevation,
            _ => return Err(anyhow::anyhow!("Failed to parse elevation")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing fix"))?;
        let fix = match part.parse::<u16>() {
            Ok(fix) => fix,
            _ => return Err(anyhow::anyhow!("Failed to parse fix")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing speed"))?;
        let speed = match part.parse::<f32>() {
            Ok(speed) => speed,
            _ => return Err(anyhow::anyhow!("Failed to parse speed")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing angle"))?;
        let angle = match part.parse::<f32>() {
            Ok(angle) => angle,
            _ => return Err(anyhow::anyhow!("Failed to parse angle")),
        };

        let part = iter.next().ok_or(anyhow::anyhow!("Missing data count"))?;
        let data_count: usize = match part.parse::<u16>() {
            Ok(data_count) => data_count as usize,
            _ => return Err(anyhow::anyhow!("Failed to parse data count")),
        };

        let mut data = Vec::<i16>::new();
        let mut sum = 0u64;
        for _ in 10..10usize + data_count {
            let part = iter.next().ok_or(anyhow::anyhow!("Missing data"))?;
            let value = match part.parse::<i16>() {
                Ok(value) => value,
                _ => return Err(anyhow::anyhow!("Failed to parse data")),
            };

            sum += value as u64;
            data.push(value);
        }

        let checksum =
            atoi::atoi::<u64>(iter.next().ok_or(anyhow::anyhow!("Missing checksum"))?.as_bytes()).unwrap();

        if checksum != sum {
            return Err(anyhow::anyhow!("Checksum failed"));
        }

        let frame = Frame {
            timestamp: timestamp,
            sample_rate: sample_rate,
            metadata: metadata,
            latitude: latitude,
            longitude: longitude,
            elevation: elevation,
            fix: fix,
            speed: speed,
            angle: angle,
            data: data,
        };

        return Ok(frame);
    }


    pub fn timestamp(&self) -> Option<i64> {
        return self.timestamp
    }

    pub fn satellite_count(&self) -> u16 {
        return self.fix
    }

    pub fn samples(&self) -> Vec<i16> {
        return self.data.clone();
    }


}