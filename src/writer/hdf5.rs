use std::{path::{Path, PathBuf}, str::FromStr};

use chrono::Utc;
use hdf5::types::{FixedUnicode, VarLenUnicode};
use ndarray::{arr2, s, Array2, Array1};

use super::Writer;

#[macro_export]
macro_rules! a_dataset {
    ($file:expr, $name:expr, $dtype:ty, $shape:expr, $chunk:expr) => {
        $file.new_dataset::<$dtype>()
            .chunk($chunk)
            .shape($shape)
            .create($name)?
    };
}

#[derive(Clone)]
pub struct HDF5WriterConfig {
    pub node_id: String,
    pub output_path: PathBuf,
    pub gzip_level: i8
}

pub struct HDF5Writer {
    output_path: PathBuf,
    file: hdf5::File,
    ds_gps_time: hdf5::Dataset,
    ds_cpu_time: hdf5::Dataset,
    ds_latitude: hdf5::Dataset,
    ds_longitude: hdf5::Dataset,
    ds_elevation: hdf5::Dataset,
    ds_satellites: hdf5::Dataset,
    ds_comments: hdf5::Dataset,
    data_set_samples: hdf5::Dataset,
    index: usize
}


impl HDF5Writer {

}

impl Writer<HDF5WriterConfig> for HDF5Writer {
    async fn write_frame(&mut self, when: chrono::DateTime<Utc>, frame: &crate::serial::Frame) -> anyhow::Result<()> {
        log::debug!("Writing frame to HDF5 file at index: {}", self.index);

        // Resize the dataset to fit the new data
        self.ds_gps_time.resize([self.index + 1])?;

        // Write the new data
        self.ds_gps_time.write_slice(
            &[frame.timestamp().ok_or(anyhow::anyhow!("No timestamp"))?],
            &[self.index]
        )?;

        self.ds_cpu_time.resize([self.index + 1])?;
        self.ds_cpu_time.write_slice(
            &[when.timestamp()],
            &[self.index]
        )?;

        self.ds_latitude.resize([self.index + 1])?;
        self.ds_latitude.write_slice(
            &[frame.latitude()],
            &[self.index]
        )?;

        self.ds_longitude.resize([self.index + 1])?;
        self.ds_longitude.write_slice(
            &[frame.longitude()],
            &[self.index]
        )?;

        self.ds_elevation.resize([self.index + 1])?;
        self.ds_elevation.write_slice(
            &[frame.elevation()],
            &[self.index]
        )?;

        self.ds_satellites.resize([self.index + 1])?;
        self.ds_satellites.write_slice(
            &[frame.satellite_count() as i8],
            &[self.index]
        )?;

        self.data_set_samples.resize([self.index + 1, 7200])?;
        self.data_set_samples.write_slice(&frame.samples(), (self.index, ..))?;

        self.file.flush()?;

        self.index += 1;

        Ok(())
    }

    fn new(config: HDF5WriterConfig)-> anyhow::Result<HDF5Writer> {
        let file = hdf5::File::create(config.output_path.join(Path::new(format!("{}_{}.h5", config.node_id, chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S")).as_str())))?;

        let attr = file.new_attr::<VarLenUnicode>().create("NODE_ID")?;
        let varlen = hdf5::types::VarLenUnicode::from_str(&config.node_id).unwrap();
        attr.write_scalar(&varlen)?;


        let data_set_sample = file.new_dataset::<i16>()
            .chunk(7200)
            .shape(7200)
            .create("sample")?;

        // write sample indicies
        let sample = Array1::from_shape_fn(7200, |i| i as i16);
        data_set_sample.write_slice(sample.as_slice().unwrap(), ..)?;

        let ds_gps_time = a_dataset!(file, "gps_time", i64, [0..], 1);
        let ds_cpu_time = a_dataset!(file, "cpu_time", i64, [0..], 1);
        let ds_latitude = a_dataset!(file, "latitude", f32, [0..], 1);
        let ds_longitude = a_dataset!(file, "longitude", f32, [0..], 1);
        let ds_elevation = a_dataset!(file, "elevation", f32, [0..], 1);
        let ds_satellites = a_dataset!(file, "satellites", i8, [0..], 1);

        let ds_comments = file.new_dataset::<VarLenUnicode>()
            .chunk(1)
            .deflate(8)
            .shape(0..)
            .create("comments")?;

        let comment = hdf5::types::VarLenUnicode::from_str("You found the comments! Any messages obtained from the Teensy board will appear here.").unwrap();
        ds_comments.resize([ds_comments.size() + 1])?;
        ds_comments.write_slice(&[comment], &[ds_comments.size() - 1])?;

        let data_set_samples = file.new_dataset::<i16>()
            .chunk((1, 7200))
            .shape((0.., 7200))
            .deflate(config.gzip_level as u8)
            .create("samples")?;

        Ok(HDF5Writer {
            output_path: config.output_path,
            file,
            ds_gps_time,
            ds_cpu_time,
            ds_latitude,
            ds_longitude,
            ds_elevation,
            ds_satellites,
            ds_comments,
            data_set_samples: data_set_samples,
            index: 0
        })
    }
    
    fn close(self) -> anyhow::Result<()> {
        self.file.flush()?;
        self.file.close()?;
        Ok(())
    }
    
    async fn write_comment(&mut self, comment: &str) -> anyhow::Result<()> {
        let comment = hdf5::types::VarLenUnicode::from_str(comment).unwrap();
        self.ds_comments.resize([self.ds_comments.size() + 1])?;
        self.ds_comments.write_slice(&[comment], &[self.ds_comments.size() - 1])?;
        Ok(())
    }
}
