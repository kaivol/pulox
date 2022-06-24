use std::future::Future;
use std::pin::Pin;

use contec_protocol::incoming_package::RealTimeData;
use csv_async::{AsyncWriter, AsyncWriterBuilder};
use futures::future::FutureExt;
use tokio::fs::{File, OpenOptions};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub trait OutputMode {
    type DataType;
    const HEADER: &'static [&'static str];

    fn format(data: Self::DataType) -> Vec<String>;
}

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

pub trait OutputWriter<T: OutputMode> {
    fn init<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>;

    fn write_record<'a>(
        &'a mut self,
        data: T::DataType,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>
    where
        <T as OutputMode>::DataType: 'a;
}

pub struct CsvWriter {
    csv: AsyncWriter<Compat<File>>,
}

impl CsvWriter {
    pub async fn new(path: String) -> anyhow::Result<Self> {
        let file = OpenOptions::new().write(true).truncate(true).create(true).open(path).await?;
        let csv = AsyncWriterBuilder::new().create_writer(file.compat_write());
        Ok(Self { csv })
    }
}

impl<T: OutputMode> OutputWriter<T> for CsvWriter {
    fn init<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        async {
            self.csv.write_record(T::HEADER).await?;
            Ok(())
        }
        .boxed_local()
    }

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
