use std::cmp::min;
use std::ffi::CStr;
use std::mem::size_of;
use std::task::Poll;
use std::task::Poll::{Pending, Ready};
use std::{ptr, slice};

use contec_protocol::incoming_package::{IncomingPackage, IncomingStateMachine};
use contec_protocol::outgoing_package;
use contec_protocol::outgoing_package::{ControlCommand, OutgoingPackage};
use mex::bindings::{
    mexCallMATLABWithTrap_800, mxArray, mxClassID_mxUINT64_CLASS, mxClassID_mxUINT8_CLASS,
    mxComplexity_mxREAL, mxCreateNumericArray_800, mxCreateNumericMatrix_800, mxCreateString_800,
    mxGetClassName_800, mxGetDoubles_800, mxGetNumberOfElements_800, mxGetProperty_800,
    mxGetUint64s_800, mxGetUint8s_800, mxIsClass_800, mxIsUint64_800, mxIsUint8_800,
    mxSetProperty_800,
};
use snafu::{ensure_whatever, whatever, Whatever};

use crate::mx_array::{create_array, get_value};

pub mod mx_array;

type Result<T, E = Whatever> = std::result::Result<T, E>;

macro_rules! mx_string {
    ($lit:expr) => {
        cstr::cstr!($lit).as_ptr() as *const std::os::raw::c_char
    };
}

mex::mex_function!(main);
fn main([action, obj]: [*mut mxArray; 2]) -> Result<[*mut mxArray; 1]> {
    let action = unsafe {
        let num_elements = mxGetNumberOfElements_800(action);
        ensure_whatever!(
            num_elements == 1,
            "Expected first argument to be a single string, found {num_elements} elements",
        );
        ensure_whatever!(mxIsUint64_800(action), "Expected first argument to be of type uint64");
        *mxGetUint64s_800(action)
    };

    let result = match action {
        // Return ContinuousRealTimeData package as bytes
        0 => get_package_bytes(ControlCommand::ContinuousRealTimeData),
        // Return StopRealTimeData package as bytes
        1 => get_package_bytes(ControlCommand::StopRealTimeData),
        // Return InformDeviceConnected package as bytes
        2 => get_package_bytes(ControlCommand::InformDeviceConnected),
        // Return initial state machine
        3 => {
            let state_machine = IncomingStateMachine::None;

            unsafe {
                let result = mxCreateNumericMatrix_800(
                    1,
                    size_of::<IncomingStateMachine>(),
                    mxClassID_mxUINT8_CLASS,
                    mxComplexity_mxREAL,
                );
                ptr::copy_nonoverlapping(&state_machine, mxGetUint8s_800(result) as _, 1);
                result
            }
        }
        // Resume state machine
        4 => {
            let matlab = unsafe {
                let num_elements = mxGetNumberOfElements_800(obj);
                ensure_whatever!(
                    num_elements == 1,
                    "Expected second argument to be a single object, {}",
                    num_elements
                );
                ensure_whatever!(
                    mxIsClass_800(obj, mx_string!(b"Pulox")),
                    "Expected second argument to be of type 'Pulox'"
                );
                obj
            };

            let port = unsafe {
                let port = mxGetProperty_800(matlab, 0, mx_string!(b"port"));
                let name = mxGetClassName_800(port);
                ensure_whatever!(
                    mxIsClass_800(port, mx_string!(b"internal.Serialport")),
                    "Expected property 'port' to be of type 'internal.Serialport', got {:?}",
                    CStr::from_ptr(name),
                );
                port
            };
            let state_machine_buffer = unsafe {
                let state = mxGetProperty_800(matlab, 0, mx_string!(b"state"));
                ensure_whatever!(
                    mxIsUint8_800(state),
                    "Expected property 'state' to be of type 'uint8'"
                );
                ensure_whatever!(
                    mxGetNumberOfElements_800(state) == size_of::<IncomingStateMachine>(),
                    "Expected property 'state' to of size {}",
                    size_of::<IncomingStateMachine>()
                );
                state
            };
            let state_machine: &mut IncomingStateMachine =
                unsafe { &mut (*(mxGetUint8s_800(state_machine_buffer) as *mut _)) };

            let result: Poll<Result<IncomingPackage, contec_protocol::Error<snafu::Whatever>>> =
                state_machine.resume(|buf| unsafe {
                    let num_bytes_property =
                        mxGetProperty_800(port, 0, mx_string!(b"NumBytesAvailable"));
                    let num_bytes = get_value::<f64>(num_bytes_property)
                        .map_or_else(|_| get_value::<u64>(num_bytes_property), |d| Ok(d as u64))?;
                    let num_bytes = min(num_bytes, buf.len() as u64);

                    if num_bytes == 0 {
                        return Pending;
                    }

                    let mut lhs = [ptr::null_mut(); 1];

                    let count = mxCreateNumericMatrix_800(
                        1,
                        1,
                        mxClassID_mxUINT64_CLASS,
                        mxComplexity_mxREAL,
                    );
                    *mxGetUint64s_800(count) = num_bytes;
                    let datatype = mxCreateString_800(mx_string!(b"uint8"));
                    let mut rhs = [port, count, datatype];

                    let err = mexCallMATLABWithTrap_800(
                        lhs.len() as _,
                        lhs.as_mut_ptr(),
                        rhs.len() as _,
                        rhs.as_mut_ptr(),
                        mx_string!(b"read"),
                    );
                    if !(err.is_null()) {
                        return Err(snafu::FromString::without_source(
                            "'Read' function failed".to_string(),
                        ))
                        .into();
                    }

                    let result = mxGetDoubles_800(lhs[0]);
                    let slice = slice::from_raw_parts(result, num_bytes as usize);
                    for (index, byte) in slice.iter().enumerate() {
                        buf[index] = *byte as u8;
                    }

                    Ready(Ok(num_bytes as usize))
                });
            unsafe {
                mxSetProperty_800(matlab, 0, mx_string!(b"state"), state_machine_buffer);
            }

            match result {
                Pending => create_array([0u8]),
                Ready(Ok(IncomingPackage::FreeFeedback(_))) => create_array([1u8]),
                Ready(Ok(IncomingPackage::RealTimeData(sample))) => create_array([
                    sample.probe_errors as u8,
                    sample.spo2,
                    sample.pulse_rate,
                    sample.pulse_waveform,
                ]),
                Ready(Ok(p)) => {
                    whatever!("Unexpected package {p:?}")
                }
                Ready(Err(e)) => {
                    return Err(snafu::FromString::with_source(
                        Box::new(e) as _,
                        "Error while reading data".to_string(),
                    ))
                }
            }
        }
        _ => whatever!("Unexpected action '{action}'"),
    };
    Ok([result])
}

// Get mxArray containing byte representation of package
fn get_package_bytes<P>(package: P) -> *mut mxArray
where
    P: OutgoingPackage,
{
    let bytes = outgoing_package::bytes_from_package(package);
    unsafe {
        let array =
            mxCreateNumericArray_800(1, [9].as_ptr(), mxClassID_mxUINT8_CLASS, mxComplexity_mxREAL);
        ptr::copy_nonoverlapping(bytes.as_ptr(), mxGetUint8s_800(array), bytes.len());
        array
    }
}
