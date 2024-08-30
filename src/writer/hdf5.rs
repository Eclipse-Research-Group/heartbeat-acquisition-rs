use std::{path::PathBuf, str::FromStr};

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

pub struct HDF5Writer {
    output_path: PathBuf,
    file: hdf5::File,
    ds_gps_time: hdf5::Dataset,
    ds_cpu_time: hdf5::Dataset,
    ds_latitude: hdf5::Dataset,
    ds_longitude: hdf5::Dataset,
    ds_elevation: hdf5::Dataset,
    ds_satellites: hdf5::Dataset,
    data_set_samples: hdf5::Dataset,
    index: usize
}


impl HDF5Writer {

}

impl Writer for HDF5Writer {
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

    fn new(node_id: String, path: PathBuf)-> anyhow::Result<HDF5Writer> {
        let file = hdf5::File::create(path.clone())?;

        let attr = file.new_attr::<VarLenUnicode>().create("NODE_ID")?;
        let varlen = hdf5::types::VarLenUnicode::from_str(&node_id).unwrap();
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
        let ds_altitude = a_dataset!(file, "elevation", f32, [0..], 1);
        let ds_satellites = a_dataset!(file, "satellites", i8, [0..], 1);


        let data_set_samples = file.new_dataset::<i16>()
            .chunk((1, 7200))
            .shape((0.., 7200))
            .deflate(4)
            .create("samples")?;

        Ok(HDF5Writer {
            output_path: path,
            file,
            ds_gps_time,
            ds_cpu_time,
            ds_latitude,
            ds_longitude,
            ds_elevation: ds_altitude,
            ds_satellites,
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
        // log::warn!("Writing comment to HDF5 file: {}", comment);
        Ok(())
    }
}