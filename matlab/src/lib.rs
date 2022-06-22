use std::mem::size_of;
use std::os::raw::c_int;
use std::time::Duration;

use anyhow::{ensure, Context, Error};
use contec_protocol::incoming_package::IncomingPackage;
use contec_protocol::outgoing_package::ControlCommand;
use contec_protocol::PulseOximeter;
use futures::{AsyncRead, AsyncWrite, FutureExt};
use tokio::time::Instant;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::bindings::{
    mexErrMsgTxt_800, mexPrintf_800, mxArray, mxClassID_mxINT8_CLASS, mxClassID_mxUINT8_CLASS,
    mxComplexity_mxREAL, mxCreateNumericArray_800, mxGetUint8s_800, mxMalloc_800, mxSetN_800,
    mxSetUint8s_800, mxUint8, size_t,
};

mod bindings;

macro_rules! mx_println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        let args = format!($($arg)*);
        #[allow(unused_unsafe)]
        unsafe { mexPrintf_800(format!("{}\n\0", args).as_bytes() as *const _ as _) };
    }};
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
) -> anyhow::Result<IncomingPackage> {
    match tokio::time::timeout(Duration::from_secs(1), device.receive_package()).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(Error::from(err)),
        Err(_) => Err(Error::msg("Device did not send a response")),
    }
}

#[allow(unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn mexFunction(
    nlhs: c_int,
    plhs: *mut *mut mxArray,
    nrhs: c_int,
    prhs: *mut *mut mxArray,
) {
    if let Err(err) = main(nlhs, plhs, nrhs, prhs) {
        mexErrMsgTxt_800(format!("{}\0", err).as_bytes() as *const _ as _);
    }
}

fn main(
    nlhs: c_int,
    plhs: *mut *mut mxArray,
    nrhs: c_int,
    prhs: *mut *mut mxArray,
) -> anyhow::Result<()> {
    ensure!(
        nlhs == 1 || nlhs == 0,
        "Number of output arguments should be 1, but it is {nlhs}"
    );
    ensure!(nrhs == 0, "Number of input arguments should be 0, but it is {nrhs}");

    let result = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?
        .block_on(main_async())?;
    unsafe {
        *plhs = result;
    }

    Ok(())
}

async fn main_async() -> anyhow::Result<*mut mxArray> {
    let port = tokio_serial::SerialStream::open(&tokio_serial::new("COM3", 115200))
        .context("Could not connect to device")?;

    let mut device = &mut PulseOximeter::new(port.compat());

    // Send StopRealTimeData and wait for FreeFeedback response
    device.send_package(ControlCommand::StopRealTimeData).await?;
    loop {
        // Ignore unexpected packages
        if let IncomingPackage::FreeFeedback(_) = receive_package_with_timeout(&mut device).await? {
            break;
        }
    }

    // Request real time data
    device.send_package(ControlCommand::ContinuousRealTimeData).await?;

    let mut interval =
        tokio::time::interval_at(Instant::now() + Duration::from_secs(5), Duration::from_secs(5));

    const NUM_TIMESTEPS: usize = 100;
    let data = unsafe { mxMalloc_800((3 * NUM_TIMESTEPS * size_of::<mxUint8>()) as size_t) }
        as *mut mxUint8;
    let mut row: usize = 0;
    loop {
        futures::select! {
            // Send InformDeviceConnected every 5 seconds
            _ = interval.tick().fuse() => {
                device.send_package(ControlCommand::InformDeviceConnected).await?;
            },
            // Read incoming packages
            package = receive_package_with_timeout(device).fuse() => {
                match package? {
                    IncomingPackage::RealTimeData(d) => {
                        unsafe {
                            data.add(row*2).write(d.probe_errors);
                            data.add(row*2+1).write(d.pulse_rate);
                            data.add(row*2+2).write(d.spo2);
                        }
                        row += 1;
                        if row == NUM_TIMESTEPS {
                            break;
                        }
                    },
                    p => anyhow::bail!("Unexpected Package {p:?}"),
                }
            }
        }
    }

    // Stop real time data
    device.send_package(ControlCommand::StopRealTimeData).await?;

    unsafe {
        let output = mxCreateNumericArray_800(
            2,
            &[3, NUM_TIMESTEPS as u64] as *const _,
            mxClassID_mxUINT8_CLASS,
            mxComplexity_mxREAL,
        );
        mxSetUint8s_800(output, data);
        Ok(output)
    }
}
