use std::mem::size_of;
use std::os::raw::c_int;
use std::panic::catch_unwind;
use std::time::Duration;

use anyhow::{ensure, Context, Error};
use contec_protocol::incoming_package::IncomingPackage;
use contec_protocol::outgoing_package::ControlCommand;
use contec_protocol::PulseOximeter;
use futures::{AsyncRead, AsyncWrite, FutureExt};
use tokio::time::Instant;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::bindings::{
    mexErrMsgTxt_800, mxArray, mxClassID_mxUINT8_CLASS, mxComplexity_mxREAL,
    mxCreateNumericArray_800, mxMalloc_800, mxSetUint8s_800, mxUint8, size_t,
};

mod bindings;
mod util;

async fn receive_package_with_timeout<T: AsyncRead + AsyncWrite + Unpin>(
    device: &mut PulseOximeter<T>,
) -> anyhow::Result<IncomingPackage> {
    match tokio::time::timeout(Duration::from_secs(1), device.receive_package()).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(Error::from(err)),
        Err(_) => Err(Error::msg("Device did not send a response")),
    }
}

/// # Safety
/// Function is called by Matlab
#[no_mangle]
pub unsafe extern "C" fn mexFunction(
    nlhs: c_int,
    plhs: *mut *mut mxArray,
    nrhs: c_int,
    prhs: *mut *mut mxArray,
) {
    match catch_unwind(|| main(nlhs, plhs, nrhs, prhs)) {
        Err(err) => {
            if let Some(s) = err.downcast_ref::<&str>() {
                mexErrMsgTxt_800(format!("{}\0", s).as_bytes() as *const _ as _);
            }
            if let Some(s) = err.downcast_ref::<String>() {
                mexErrMsgTxt_800(format!("{}\0", s).as_bytes() as *const _ as _);
            }
        }
        Ok(Err(err)) => mexErrMsgTxt_800(format!("{}\0", err).as_bytes() as *const _ as _),
        Ok(Ok(())) => {}
    }
}

fn main(
    nlhs: c_int,
    plhs: *mut *mut mxArray,
    nrhs: c_int,
    _prhs: *mut *mut mxArray,
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

    let device = &mut PulseOximeter::new(port.compat());

    // Send StopRealTimeData and wait for FreeFeedback response
    device.send_package(ControlCommand::StopRealTimeData).await?;
    loop {
        // Ignore unexpected packages
        if let IncomingPackage::FreeFeedback(_) = receive_package_with_timeout(device).await? {
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
                            data.add(row*3).write(if d.probe_errors { 1 } else { 0 });
                            data.add(row*3+1).write(d.pulse_rate);
                            data.add(row*3+2).write(d.spo2);
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
