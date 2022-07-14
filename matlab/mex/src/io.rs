use core::fmt::Arguments;

use crate::bindings::mexPrintf_800;

#[doc(hidden)]
pub fn mx_print(args: Arguments) {
    unsafe { mexPrintf_800(format!("{}\0", args).as_bytes() as *const _ as _) };
}

#[macro_export]
macro_rules! mx_print {
    ($($arg:tt)*) => {
        $crate::io::mx_print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! mx_println {
    () => {
        $crate::mx_print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::io::mx_print(core::format_args!("{}\n", core::format_args!($($arg)*)));
    }};
}

#[macro_export]
macro_rules! mx_dbg {
    () => {
        $crate::mx_println!("[{}:{}]", core::file!(), core::line!())
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::mx_println!("[{}:{}] {} = {:#?}",
                    core::file!(), core::line!(), core::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::mx_dbg!($val)),+,)
    };
}
