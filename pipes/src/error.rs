use alloc::boxed::Box;
use core::fmt;

pub type BoxError = Box<dyn core::error::Error + Send + Sync>;

pub type Result<T> = core::result::Result<T, Error>;

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        write!(f, "{}", self.inner)?;
        #[cfg(not(feature = "std"))]
        write!(f, "{:?}", self.inner)?;
        Ok(())
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}
