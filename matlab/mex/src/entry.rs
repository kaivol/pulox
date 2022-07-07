use std::ptr;

use crate::bindings::mxArray;
use crate::Error;

#[macro_export]
macro_rules! mex_function {
    ($function:path) => {
        /// # Safety
        /// Function is called by Matlab
        #[no_mangle]
        pub unsafe extern "C" fn mexFunction(
            nlhs: std::os::raw::c_int,
            plhs: *mut *mut $crate::bindings::mxArray,
            nrhs: std::os::raw::c_int,
            prhs: *mut *mut $crate::bindings::mxArray,
        ) {
            $crate::entry::check_arguments_and_call(nlhs, plhs, nrhs, prhs, $function)
        }
    };
}

#[doc(hidden)]
#[inline]
pub unsafe fn check_arguments_and_call<const RHS: usize, const LHS: usize, E: Into<Error>>(
    nlhs: std::os::raw::c_int,
    plhs: *mut *mut mxArray,
    nrhs: std::os::raw::c_int,
    prhs: *mut *mut mxArray,
    f: fn([*mut mxArray; RHS]) -> std::result::Result<[*mut mxArray; LHS], E>,
) {
    if let Err(err) = (|| {
        if (nrhs as usize) > RHS {
            return Err(Error::new("TooManyInputs", "Too many inputs"));
        }
        if (nrhs as usize) < RHS {
            return Err(Error::new("TooFewInputs", "Too few inputs"));
        }
        if (nlhs as usize) > LHS {
            return Err(Error::new("TooManyOutputs", "Too many outputs"));
        }
        if (nlhs as usize) < LHS {
            return Err(Error::new("TooFewOutputs", "Too few outputs"));
        }
        let inputs: [*mut mxArray; RHS] = *(prhs as *mut [*mut mxArray; RHS]);
        let outputs: &mut [*mut mxArray; LHS] = &mut *(plhs as *mut [*mut mxArray; LHS]);

        let result = match std::panic::catch_unwind(|| f(inputs)) {
            Ok(Err(err)) => return Err(err.into()),
            Ok(Ok(result)) => result,
            Err(err) => {
                return if let Some(s) = err.downcast_ref::<String>() {
                    Err(Error::new("Panic", s))
                } else if let Some(s) = err.downcast_ref::<&str>() {
                    Err(Error::new("Panic", *s))
                } else {
                    Err(Error::new("Panic", "Panic: unknown reason"))
                }
            }
        };
        ptr::copy_nonoverlapping(result.as_ptr(), outputs.as_mut_ptr(), LHS);
        Ok(())
    })() {
        err.throw()
    };
}
