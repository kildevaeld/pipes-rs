#![no_std]

use core::fmt;

use alloc::{borrow::Cow, boxed::Box, collections::btree_map::BTreeMap};

extern crate alloc;

pub type BoxError = Box<dyn core::error::Error + Send + Sync>;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: BoxError,
    values: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
}

impl Error {
    pub fn new<T: Into<BoxError>>(error: T) -> Error {
        Error {
            inner: error.into(),
            values: Default::default(),
        }
    }

    pub fn value(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        if !self.values.is_empty() {
            write!(f, " [")?;
            for (key, value) in self.values.iter() {
                write!(f, "{}: {}, ", key, value)?;
            }
            write!(f, "]")?;
        }

        Ok(())
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.inner.source()
    }
}

impl From<BoxError> for Error {
    fn from(value: BoxError) -> Self {
        Error::new(value)
    }
}
