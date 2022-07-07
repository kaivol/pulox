use std::ffi::CStr;
use std::os::raw::c_char;

#[inline]
pub fn abort_with_id(id: impl AsRef<CStr>, message: impl AsRef<str>) -> ! {
    let id: *const c_char = id.as_ref().as_ptr() as _;
    let message = message.as_ref().as_bytes();
    unsafe {
        crate::bindings::mexErrMsgIdAndTxt_800(
            id,
            b"%.*s\0".as_ptr() as *const _,
            message.len(),
            message.as_ptr() as *const c_char,
        );
    }
    #[allow(clippy::empty_loop)]
    loop {}
}

#[inline]
pub fn abort(message: impl AsRef<CStr>) -> ! {
    let message = message.as_ref();
    unsafe {
        crate::bindings::mexErrMsgTxt_800(message.as_ptr() as _);
    }
    #[allow(clippy::empty_loop)]
    loop {}
}
