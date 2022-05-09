use thiserror::Error;

/// A specialized `Error` type that provides device communcation error information.
#[derive(Error, Debug)]
pub enum Error {
    /// communicating with the device failed
    #[error("communicating with the device failed: {0:?}")]
    DeviceIOError(#[from] futures::io::Error),

    /// device reported `0` bytes written
    #[error("device reported '0' bytes written")]
    DeviceWriteZero,

    /// devices reported more bytes written than requested
    #[error("tried to write '{requested}' bytes, but device reported '{reported}' bytes written")]
    DeviceWriteTooMuch {
        /// number of requested bytes
        requested: usize,
        /// number of reportedly written bytes
        reported: usize,
    },

    /// device reported '0' bytes read
    #[error("device reported '0' bytes read")]
    DeviceReadZero,

    /// devices reported more bytes read than requested
    #[error("tried to read '{requested}' bytes, but device reported '{reported}' bytes read")]
    DeviceReadTooMuch {
        /// number of requested bytes
        requested: usize,
        /// number of reportedly read bytes
        reported: usize,
    },

    /// invalid package
    #[error(
        "synchronization bit of byte '{:02X?}' at index '{}' must be set. Raw package: '{:02X?}' {:02X?}",
        bytes[*invalid_index],
        invalid_index,
        code,
        &bytes[..*length]
    )]
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
    #[error("got unknown package type code {0:#04X}")]
    UnknownTypeCode(u8),
}
