use anyhow::{Context, Error, Result};
use clap::Parser;
use contec_protocol::{
    incoming_package::IncomingPackage, outgoing_package::ControlCommand, PulseOximeter,
};
use futures::{Future, FutureExt};
use std::io::stdout;
use std::io::Write;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::compat::TokioAsyncReadCompatExt;

#[derive(Parser, Debug)]
#[clap(author, version,
    about = "Read data from Pulox PPG",
    long_about = None
)]
struct Cli {
    /// Name of serial port
    #[clap(short, long, default_value = "COM3")]
    port: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let port = tokio_serial::SerialStream::open(&tokio_serial::new(cli.port, 115200))
        .context("Could not connect to device")?;

    let mut device = PulseOximeter::new(port.compat());

    // Send StopRealTimeData and wait for FreeFeedback response
    device.send_package(ControlCommand::StopRealTimeData).await?;
    loop {
        // Ignore unexpected packages
        if let IncomingPackage::FreeFeedback(_) = timeout(device.receive_package()).await? {
            break;
        }
    }

    // Read device id
    device.send_package(ControlCommand::AskForDeviceIdentifier).await?;
    match timeout(device.receive_package()).await? {
        IncomingPackage::DeviceIdentifier(i) => {
            println!(
                "Device identifier is '{}'",
                core::str::from_utf8(&i.identifier).context("Received invalid identifier")?
            );
        }
        p => return Err(Error::msg(format!("Unexpected Package {p:?}"))),
    }

    // Request real time data
    device.send_package(ControlCommand::ContinuousRealTimeData).await?;

    let mut interval = tokio::time::interval_at(
        Instant::now() + Duration::from_secs(5),
        Duration::from_secs(5),
    );
    println!("Press Ctrl-C to cancel");
    println!();
    println!("Error │ SpO2 │ Pulse ");
    println!("══════╪══════╪══════════════════════");
    loop {
        futures::select! {
            // Listen for Ctrl-C
            _ = tokio::signal::ctrl_c().fuse() => {
                eprintln!("\nGot Ctrl-C. Exiting");
                break;
            },
            // Send InformDeviceConnected every 5 seconds
            _ = interval.tick().fuse() => {
                device.send_package(ControlCommand::InformDeviceConnected).await?;
            },
            // Read incoming packages
            package = timeout(device.receive_package()).fuse() => {
                match package? {
                    IncomingPackage::RealTimeData(d) => {
                        print!("\r{:5} │ {:4} │ {:5} {}{}",
                            d.probe_errors,
                            d.spo2,
                            d.pulse_rate,
                            "■".repeat(d.bar_graph.into()),
                            " ".repeat((15 - d.bar_graph).into()),
                        );
                        stdout().flush()?;
                    },
                    p => return Err(Error::msg(format!("Unexpected Package {p:?}"))),
                }
            }
        }
    }

    // Stop real time data
    device.send_package(ControlCommand::StopRealTimeData).await?;

    Ok(())
}

async fn timeout<T, E>(fut: impl Future<Output = Result<T, E>>) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
{
    match tokio::time::timeout(Duration::from_secs(1), fut).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(Error::from(err)),
        Err(_) => Err(Error::msg("Device did not send a response")),
    }
}
