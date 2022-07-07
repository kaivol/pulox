pub mod abort;

mod allocator;

pub mod bindings;

pub mod entry;

mod error;
pub use error::Error;

pub mod io;

pub type Result<T> = core::result::Result<T, Error>;
