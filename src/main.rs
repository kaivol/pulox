mod output;
mod realtime;

use std::fmt;
use std::time::Duration;

use anyhow::{anyhow, bail, ensure, Context, Error, Result};
use chrono::{Datelike, Local, Timelike};
use clap::{ArgEnum, Args, Parser, Subcommand};
use contec_protocol::incoming_package::IncomingPackage;
use contec_protocol::outgoing_package::ControlCommand;
use contec_protocol::PulseOximeter;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use futures::{AsyncRead, AsyncWrite, FutureExt};
use realtime::GraphTerminal;
use tokio::time;
use tokio::time::Instant;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::output::{CsvWriter, OutputMode, OutputWriter, Realtime, Storage};
use crate::realtime::{MinTerminal, RealtimeTerminal};

#[derive(Parser, Debug)]
#[clap(author, version,
    about = "Interact with Pulox PPG",
    long_about = None
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// Name of serial port
    #[clap(default_value = "COM3")]
    port: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Read real time data
    Realtime(RealtimeArgs),

    /// Read storage data
    Storage(StorageArgs),

    /// Delete storage data segment
    ClearStorage,

    /// Sync device time
    SyncTime,
}

#[derive(Args, Debug)]
struct RealtimeArgs {
    /// Output format
    #[clap(long, short, arg_enum, value_parser, requires = "output")]
    format: Option<OutputFormat>,
    /// Output File
    #[clap(long, short, requires = "format")]
    output: Option<String>,
    /// Show no output in console
    #[clap(long)]
    no_console: bool,
}

#[derive(Args, Debug)]
struct StorageArgs {
    /// Output format
    #[clap(long, short, arg_enum, value_parser)]
    format: OutputFormat,
    /// Output File
    #[clap(long, short)]
    output: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum OutputFormat {
    Csv,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            OutputFormat::Csv => "csv",
        })
    }
}

impl OutputFormat {
    pub async fn get_writer<T: OutputMode>(
        &self,
        args: String,
    ) -> Result<Box<dyn OutputWriter<T>>> {
        let mut writer = match self {
            OutputFormat::Csv => Box::new(CsvWriter::new(args).await?),
        };
        OutputWriter::<T>::init(writer.as_mut()).await?;
        Ok(writer)
    }
}

macro_rules! expect_package_with_timeout {
    ($device:ident, $package:tt) => {
        async {
            match receive_package_with_timeout($device).await? {
                IncomingPackage::$package(i) => Ok(i),
                p => Err(anyhow!("Unexpected Package {p:?}")),
            }
        }
    };
}

async fn receive_package_with_timeout<T: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<T>,
) -> Result<IncomingPackage> {
    match time::timeout(Duration::from_secs(1), device.receive_package()).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(Error::from(err)),
        Err(_) => Err(Error::msg("Device did not send a response")),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let port = tokio_serial::SerialStream::open(&tokio_serial::new(&cli.port, 115200)).context(
        format!(
            "Could not connect to device {}.\n\
            Available ports: {}",
            cli.port,
            tokio_serial::available_ports()?
                .into_iter()
                .map(|p| p.port_name)
                .collect::<Vec<_>>()
                .join(", "),
        ),
    )?;

    let mut device = PulseOximeter::new(port.compat());

    // Send StopRealTimeData and wait for FreeFeedback response
    device.send_package(ControlCommand::StopRealTimeData).await?;
    loop {
        // Ignore unexpected packages
        if let IncomingPackage::FreeFeedback(_) = receive_package_with_timeout(&mut device).await? {
            break;
        }
    }

    match cli.command {
        Command::Realtime(args) if args.no_console => {
            realtime::<MinTerminal, _>(&mut device, args, cli.port).await
        }
        Command::Realtime(args) => realtime::<GraphTerminal, _>(&mut device, args, cli.port).await,
        Command::Storage(args) => storage(&mut device, args).await,
        Command::ClearStorage => clear_storage(&mut device).await,
        Command::SyncTime => sync_time(&mut device).await,
    }
}

async fn realtime<T: RealtimeTerminal, U: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<U>,
    args: RealtimeArgs,
    port: String,
) -> Result<()> {
    let mut terminal = T::new()?;
    terminal.add_message(format!("Connected to device {port}"))?;

    let mut writer = if let Some((format, output)) = args.format.zip(args.output) {
        terminal.add_message(format!("Saving to file {output} in {format} format"))?;
        Some(format.get_writer::<Realtime>(output).await?)
    } else {
        None
    };

    // Request real time data
    device.send_package(ControlCommand::ContinuousRealTimeData).await?;

    let mut terminal_interval = time::interval(Duration::from_millis(50));
    let mut keep_alive_interval =
        time::interval_at(Instant::now() + Duration::from_secs(5), Duration::from_secs(5));

    terminal.add_message("Press ESC to exit")?;
    loop {
        futures::select! {
            // Listen for Ctrl-C and ESC
            event = terminal.handle_event().fuse() => {
                match event {
                    Ok(event) if event == Event::Key(KeyCode::Esc.into()) => break,
                    Ok(event) if event == Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)) => break,
                    _ => {}
                }
            }
            // Send InformDeviceConnected every 5 seconds
            _ = keep_alive_interval.tick().fuse() => {
                device.send_package(ControlCommand::InformDeviceConnected).await?;
            },
            // Read incoming packages
            package = receive_package_with_timeout(device).fuse() => {
                match package? {
                    IncomingPackage::RealTimeData(d) => {
                        terminal.next_sample(d);
                        if let Some(ref mut writer) = writer {
                            writer.write_record(d).await?;
                        }
                    },
                    p => bail!("Unexpected Package {p:?}"),
                }
            }
            // Update terminal
            _ = terminal_interval.tick().fuse() => {
                terminal.update()?;
            }
        }
    }

    // Stop real time data
    terminal.clear_messages()?;
    terminal.add_message("Stop real time data")?;
    device.send_package(ControlCommand::StopRealTimeData).await?;

    terminal.close()?;

    Ok(())
}

async fn storage<T: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<T>,
    args: StorageArgs,
) -> Result<()> {
    let mut writer = args.format.get_writer::<Storage>(args.output).await?;

    let (user_index, segment_index) = get_user_and_segment(device).await?;

    // Asking for Storage start time
    device
        .send_package(ControlCommand::AskForStorageStartTime(user_index, segment_index))
        .await?;
    let d = expect_package_with_timeout!(device, StorageStartTimeDate).await?;
    let t = expect_package_with_timeout!(device, StorageStartTimeTime).await?;
    println!(
        "The storage start time is {}:{}:{} on {}.{}.{}",
        t.hour, t.minute, t.second, d.year, d.month, d.day
    );

    // Asking for data length
    device
        .send_package(ControlCommand::AskForStorageDataLength(user_index, segment_index))
        .await?;
    let data_length = expect_package_with_timeout!(device, StorageDataLength).await?.length;
    println!("The storage data length is {} bytes ({} samples)", data_length, data_length / 2);

    // Asking for storage data
    device
        .send_package(ControlCommand::AskForStorageData(user_index, segment_index))
        .await?;

    for i in (0..data_length).step_by(6) {
        let d = expect_package_with_timeout!(device, StorageData).await?;
        writer.write_record((d.spo2_1, d.pulse_rate_1)).await?;
        if i + 2 < data_length {
            writer.write_record((d.spo2_2, d.pulse_rate_2)).await?;
        }
        if i + 4 < data_length {
            writer.write_record((d.spo2_3, d.pulse_rate_3)).await?;
        }
    }
    println!("Finished reading and saving data");
    Ok(())
}

async fn clear_storage<T: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<T>,
) -> Result<()> {
    let (user_index, segment_index) = get_user_and_segment(device).await?;

    // Asking for data length
    device
        .send_package(ControlCommand::DeleteStorageData(user_index, segment_index))
        .await?;
    let feedback = expect_package_with_timeout!(device, CommandFeedback).await?;
    ensure!(feedback.code == 0, "Could not clear storage: {:?}", feedback);

    println!("Successfully deleted segment {} for user {}", segment_index, user_index);
    Ok(())
}

async fn get_user_and_segment<T: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<T>,
) -> Result<(u8, u8)> {
    // Asking for the amount of users
    device.send_package(ControlCommand::AskForUserAmount).await?;
    let user_count = expect_package_with_timeout!(device, UserAmount).await?.total_user;
    let user_index: u8 = if user_count > 1 {
        dialoguer::Input::new()
            .with_prompt(format!("Choose the user index from 0 to {}", user_count - 1))
            .interact_text()?
    } else {
        0
    };

    // Choosing the data segment
    device
        .send_package(ControlCommand::AskForStorageDataSegmentAmount(user_index))
        .await?;
    let segment_count = expect_package_with_timeout!(device, StorageDataSegmentAmount)
        .await?
        .segment_amount;
    let segment_index: u8 = if user_count > 1 {
        dialoguer::Input::new()
            .with_prompt(format!("Choose the storage data segment from 0 to {}", segment_count - 1))
            .interact_text()?
    } else {
        0
    };
    Ok((user_index, segment_index))
}

async fn sync_time<T: AsyncRead + AsyncWrite + Unpin>(device: &mut PulseOximeter<T>) -> Result<()> {
    let now = Local::now();
    let year = now.year();
    let high_year = year / 100;
    let low_year = year - high_year;

    // Sending package with current date
    device
        .send_package(ControlCommand::SynchronizeDeviceDate(
            high_year as u8,
            low_year as u8,
            now.month() as _,
            now.day() as _,
            now.weekday().num_days_from_sunday() as _,
        ))
        .await?;
    let feedback = expect_package_with_timeout!(device, CommandFeedback).await?;
    ensure!(feedback.code == 0, "Could not set device time: {:?}", feedback);

    device
        .send_package(ControlCommand::SynchronizeDeviceTime(
            now.hour() as _,
            now.minute() as _,
            now.second() as _,
        ))
        .await?;
    let feedback = expect_package_with_timeout!(device, CommandFeedback).await?;
    ensure!(feedback.code == 0, "Could not set device time: {:?}", feedback);

    println!("Successfully set device time");
    Ok(())
}
