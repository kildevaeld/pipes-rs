use core::fmt::Debug;

use alloc::boxed::Box;

#[cfg(feature = "std")]
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[cfg(not(feature = "std"))]
pub type BoxError = Box<dyn Debug + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    inner: BoxError,
}

impl Error {
    pub fn new<T: Into<BoxError>>(error: T) -> Error {
        Error {
            inner: error.into(),
        }
    }
}
