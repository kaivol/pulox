use std::future::Future;
use std::pin::Pin;

use futures::future::FutureExt;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

pub trait OutputWriter {
    fn write<'a>(
        &'a mut self,
        pulse_rate: u8,
        spo2: u8,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>;
}

pub struct CsvWriter {
    file: File,
}

impl CsvWriter {
    pub async fn new(path: String) -> anyhow::Result<CsvWriter> {
        let file = OpenOptions::new().write(true).truncate(true).create(true).open(path).await?;
        Ok(Self { file })
    }
}

impl OutputWriter for CsvWriter {
    fn write<'a>(
        &'a mut self,
        pulse_rate: u8,
        spo2: u8,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>> {
        async move {
            self.file.write_all(format!("{pulse_rate},{spo2}\n").as_bytes()).await?;
            Ok(())
        }
        .boxed_local()
    }
}
