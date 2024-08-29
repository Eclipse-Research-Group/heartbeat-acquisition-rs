use std::{path::PathBuf, str::FromStr};

use hdf5::types::{FixedUnicode, VarLenUnicode};
use ndarray::{arr2, s, Array2, Array1};

use super::Writer;

pub struct HDF5Writer {
    output_path: PathBuf,
    file: hdf5::File,
    data_set_time: hdf5::Dataset,
    data_set_samples: hdf5::Dataset,
    index: usize
}


impl HDF5Writer {

}

impl Writer for HDF5Writer {
    async fn write_frame(&mut self, frame: &crate::serial::Frame) -> anyhow::Result<()> {
        log::debug!("Writing frame to HDF5 file at index: {}", self.index);

        // Resize the dataset to fit the new data
        self.data_set_time.resize([self.index + 1])?;

        // Write the new data
        self.data_set_time.write_slice(
            &[frame.timestamp().ok_or(anyhow::anyhow!("No timestamp"))? as f64],
            &[self.index]
        )?;

        self.data_set_samples.resize([self.index + 1, 7200])?;

        self.data_set_samples.write_slice(&frame.samples(), (self.index, ..))?;

        self.file.flush()?;

        self.index += 1;

        Ok(())
    }

    fn new(path: PathBuf)-> anyhow::Result<HDF5Writer> {
        let file = hdf5::File::create(path.clone())?;

        let attr = file.new_attr::<VarLenUnicode>().create("NODE_ID")?;
        // let varlen: hdf5::types::VarLenUnicode = "asdf".parse().unwrap();
        let varlen = hdf5::types::VarLenUnicode::from_str("asdf").unwrap();
        attr.write_scalar(&varlen)?;


        let data_set_sample = file.new_dataset::<i16>()
            .chunk(7200)
            .shape(7200)
            .create("sample")?;

        // write sample indicies
        let sample = Array1::from_shape_fn(7200, |i| i as i16);
        data_set_sample.write_slice(sample.as_slice().unwrap(), ..)?;

        let data_set_time = file.new_dataset::<f64>()
            .chunk(1)
            .shape([0..]).create("time")?;

        let data_set_samples = file.new_dataset::<i16>()
            .chunk((1, 7200))
            .shape((0.., 7200))
            .deflate(4)
            .create("samples")?;

        Ok(HDF5Writer {
            output_path: path,
            file,
            data_set_time: data_set_time,
            data_set_samples: data_set_samples,
            index: 0
        })
    }
    
    fn close(self) -> anyhow::Result<()> {
        self.file.flush()?;
        self.file.close()?;
        Ok(())
    }
}