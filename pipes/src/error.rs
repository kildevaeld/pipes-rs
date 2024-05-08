#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl Error {
    pub fn new<T: Into<Box<dyn std::error::Error + Send + Sync>>>(error: T) -> Error {
        Error {
            inner: error.into(),
        }
    }
}
