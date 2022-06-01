//! Implements the communcation protocol of Contec pulse oximeters (V7.0)

#![warn(missing_docs)]

mod bit_ops;

mod pulse_oximeter;
pub use pulse_oximeter::{incoming_package, PulseOximeter};

pub mod outgoing_package;

mod error;
pub use error::Error;

/// A specialized `Result` type that provides device communcation error information.
pub type Result<T> = core::result::Result<T, Error>;
