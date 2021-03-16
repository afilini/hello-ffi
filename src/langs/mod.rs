#[cfg(feature = "c")]
pub mod c;
#[cfg(feature = "c")]
pub use c::*;

#[cfg(feature = "python")]
#[macro_use]
pub mod python;
#[cfg(feature = "python")]
pub use python::*;
