//! Implements the communcation protocol of Contec pulse oximeters (V7.0)

#![warn(missing_docs)]

mod bit_ops;

mod pulse_oximeter;
pub use pulse_oximeter::incoming_package;
pub use pulse_oximeter::PulseOximeter;

pub mod outgoing_package;
