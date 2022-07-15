use std::future::Future;
use std::pin::Pin;

use contec_protocol::incoming_package::RealTimeData;
use csv_async::{AsyncWriter, AsyncWriterBuilder};
use futures::future::FutureExt;
use tokio::fs::{File, OpenOptions};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// Measurement data output modes
pub trait OutputMode {
    /// Type of the data
    type DataType;
    /// Headers for output table
    const HEADER: &'static [&'static str];

    /// Formats the measurement data
    /// Output vector should have same length as `HEADER` slice
    fn format(data: Self::DataType) -> Vec<String>;
}

/// Realtime output data
pub struct Realtime;
impl OutputMode for Realtime {
    type DataType = RealTimeData;
    const HEADER: &'static [&'static str] = &["Probe error", "Pulse rate", "Sp02", "Pulse"];

    fn format(data: Self::DataType) -> Vec<String> {
        vec![
            data.probe_errors.to_string(),
            data.pulse_rate.to_string(),
            data.spo2.to_string(),
            data.pulse_waveform.to_string(),
        ]
    }
}

pub struct Storage;
impl OutputMode for Storage {
    type DataType = (u8, u8);
    const HEADER: &'static [&'static str] = &["Pulse rate", "Sp02"];

    fn format(data: Self::DataType) -> Vec<String> {
        vec![data.0.to_string(), data.1.to_string()]
    }
}

/// Save measurement data
pub trait OutputWriter<T: OutputMode> {
    /// Initialize writer, e.g. write header line in a csv file
    fn init<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>;

    /// Write a single measurement
    fn write_record<'a>(
        &'a mut self,
        data: T::DataType,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>
    where
        <T as OutputMode>::DataType: 'a;
}

/// Writes measurement data to a csv file
pub struct CsvWriter {
    csv: AsyncWriter<Compat<File>>,
}

impl CsvWriter {
    /// Create a new csv writer
    pub async fn new(path: String) -> anyhow::Result<Self> {
        let file = OpenOptions::new().write(true).truncate(true).create(true).open(path).await?;
        let csv = AsyncWriterBuilder::new().create_writer(file.compat_write());
        Ok(Self { csv })
    }
}

impl<T: OutputMode> OutputWriter<T> for CsvWriter {
    /// Write the csv header
    fn init<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        async {
            self.csv.write_record(T::HEADER).await?;
            Ok(())
        }
        .boxed_local()
    }

    /// Write the measurement as a csv line
    fn write_record<'a>(
        &'a mut self,
        data: T::DataType,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>
    where
        <T as OutputMode>::DataType: 'a,
    {
        async move {
            self.csv.write_record(T::format(data)).await?;
            Ok(())
        }
        .boxed_local()
    }
}
