use core::fmt::{self, Debug};

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

    pub fn inner(&self) -> &BoxError {
        &self.inner
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}
