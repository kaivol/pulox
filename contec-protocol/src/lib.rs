//! Implements the communcation protocol of Contec pulse oximeters (V7.0)
#![no_std]
#![warn(missing_docs)]

mod bit_ops;

mod error;
pub use error::Error;

mod encoding;

pub mod incoming_package;

pub mod outgoing_package;

mod pulse_oximeter;
pub use pulse_oximeter::PulseOximeter;

mod traits;
pub use traits::AsyncReadWrite;

/// A specialized `Result` type that provides device communication error information.
pub type Result<T, E> = core::result::Result<T, Error<E>>;
