//! Implements the communcation protocol of Contec pulse oximeters (V7.0)

#![warn(missing_docs)]

mod bit_ops;

mod pulse_oximeter;
pub use pulse_oximeter::incoming_package;
pub use pulse_oximeter::PulseOximeter;

pub mod outgoing_package;

mod error;
pub use error::Error;

/// A specialized `Result` type that provides device and communcation error information.
pub type Result<T> = core::result::Result<T, Error>;
