#![allow(unused)]

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
