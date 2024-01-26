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


struct DataPoint {
    timestamp: String,
    flags: DataPointFlags,
}


impl ToString for DataPoint {
    fn to_string(&self) -> String {
        let string: String = format!("{},{}", self.timestamp, self.flags.to_string());
        return string;
    }
}