use core::fmt::Debug;

use snafu::{AsErrorSource, Snafu};

/// A specialized `Error` type that provides device communication error information.
#[derive(Snafu, Debug)]
pub enum Error<#[cfg(feature = "std")] E: AsErrorSource, #[cfg(not(feature = "std"))] E> {
    /// communicating with the device failed
    #[snafu(display("communicating with the device failed"))]
    #[cfg_attr(feature = "std", snafu(context(false)))]
    DeviceIOError {
        /// device error
        #[cfg_attr(not(feature = "std"), snafu(source(false)))]
        source: E,
    },

    /// device reported `0` bytes written
    #[snafu(display("device reported '0' bytes written"))]
    DeviceWriteZero,

    /// devices reported more bytes written than requested
    #[snafu(display(
        "tried to write '{requested}' bytes, but device reported '{reported}' bytes written"
    ))]
    DeviceWriteTooMuch {
        /// number of requested bytes
        requested: usize,
        /// number of reportedly written bytes
        reported: usize,
    },

    /// device reported '0' bytes read
    #[snafu(display("device reported '0' bytes read"))]
    DeviceReadZero,

    /// devices reported more bytes read than requested
    #[snafu(display(
        "tried to read '{requested}' bytes, but device reported '{reported}' bytes read"
    ))]
    DeviceReadTooMuch {
        /// number of requested bytes
        requested: usize,
        /// number of reportedly read bytes
        reported: usize,
    },

    /// invalid package
    #[snafu(display(
        "synchronization bit of byte '{:02X?}' at index '{}' must be set. Raw package: '{:02X?}' {:02X?}",
        bytes[*invalid_index],
        invalid_index,
        code,
        &bytes[..*length]
    ))]
    InvalidPackageData {
        /// package type code
        code: u8,
        /// package bytes (incuding high byte)
        bytes: [u8; 8],
        /// package length
        length: usize,
        /// index of first invalid byte
        invalid_index: usize,
    },

    /// unexpected package type code encountered
    #[snafu(display("got unknown package type code {code:#04X}"))]
    UnknownTypeCode {
        /// unknown type code
        code: u8,
    },
}

#[cfg(not(feature = "std"))]
impl<E> From<E> for Error<E> {
    fn from(source: E) -> Self {
        Error::DeviceIOError { source }
    }
}
