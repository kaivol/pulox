#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{ptr, slice};

use mex::bindings::*;
use snafu::ensure_whatever;

// Tries to get the data of the mxArray as a Rust slice
pub fn get_slice<T: MatlabDataType>(array: *mut mxArray) -> crate::Result<&'static mut [T]> {
    ensure_whatever!(T::is_of_type(array), "array is of wrong type");
    let length = unsafe { mxGetNumberOfElements_800(array) };
    let ptr = T::get_data(array);
    Ok(unsafe { slice::from_raw_parts_mut(ptr, length) })
}

// Tries to get the data of a single element mxArray
pub fn get_value<T: MatlabDataType>(array: *mut mxArray) -> crate::Result<T> {
    let slice = get_slice::<T>(array)?;
    ensure_whatever!(slice.len() == 1, "array must contain exactly one value");
    Ok(slice[0])
}

// Create an mxArray from the given slice
pub fn create_array<T: MatlabDataType, const N: usize>(data: [T; N]) -> *mut mxArray {
    unsafe {
        let result =
            mxCreateNumericArray_800(1, [N].as_ptr(), mxClassID_mxUINT8_CLASS, mxComplexity_mxREAL);
        ptr::copy_nonoverlapping(data.as_ptr(), T::get_data(result), N);
        result
    }
}

pub trait MatlabDataType: Copy + 'static {
    const CLASS_ID: mxClassID;
    const CHECK_TYPE: unsafe extern "C" fn(*const mxArray) -> bool;
    const GET: unsafe extern "C" fn(*const mxArray) -> *mut Self;

    fn is_of_type(array: *const mxArray) -> bool {
        unsafe { Self::CHECK_TYPE(array) }
    }

    fn get_data(array: *const mxArray) -> *mut Self {
        unsafe { Self::GET(array) }
    }
}

impl MatlabDataType for u8 {
    const CLASS_ID: mxClassID = mxClassID_mxUINT8_CLASS;
    const CHECK_TYPE: unsafe extern "C" fn(*const mxArray) -> bool = mxIsUint8_800;
    const GET: unsafe extern "C" fn(*const mxArray) -> *mut Self = mxGetUint8s_800;
}

impl MatlabDataType for u64 {
    const CLASS_ID: mxClassID = mxClassID_mxUINT64_CLASS;
    const CHECK_TYPE: unsafe extern "C" fn(*const mxArray) -> bool = mxIsUint64_800;
    const GET: unsafe extern "C" fn(*const mxArray) -> *mut Self = mxGetUint64s_800;
}

impl MatlabDataType for f64 {
    const CLASS_ID: mxClassID = mxClassID_mxDOUBLE_CLASS;
    const CHECK_TYPE: unsafe extern "C" fn(*const mxArray) -> bool = mxIsDouble_800;
    const GET: unsafe extern "C" fn(*const mxArray) -> *mut Self = mxGetDoubles_800;
}
