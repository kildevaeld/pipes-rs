use alloc::{boxed::Box, format, sync::Arc};
use bycat::Work;
use bycat_error::{BoxError, Error};
use bytes::Bytes;
use futures::future::BoxFuture;
use toback::Toback;

use crate::{Content, Package};

pub struct Decode<T>(Arc<toback::Toback<T>>)
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize;

impl<T> Clone for Decode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Decode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    pub fn new() -> Decode<T> {
        Self(toback::Toback::new().into())
    }
}

impl<C, B, T> Work<C, Package<B>> for Decode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize + Send,
    B: Content + Send + 'static,
    B::Error: Into<BoxError>,
{
    type Output = Package<T>;
    type Error = Error;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _ctx: &'a C, mut package: Package<B>) -> Self::Future<'a> {
        Box::pin(async move {
            let Some(encoder) = self.0.encoder_from_path(package.path().as_str()) else {
                return Err(Error::new(format!(
                    "Encoder not found for path: {}",
                    package.path()
                )));
            };

            let body = package.content_mut().bytes().await.map_err(Error::new)?;
            let value = encoder.load(&body).map_err(Error::new)?;

            Ok(package.map(|_| async move { value }).await)
        })
    }
}

pub struct Encode<T>(Arc<Toback<T>>)
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize;

impl<T> Clone for Encode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Encode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    pub fn new() -> Encode<T> {
        Self(toback::Toback::new().into())
    }
}

impl<C, T> Work<C, Package<T>> for Encode<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize + Send,
{
    type Output = Package<Bytes>;

    type Error = Error;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _ctx: &'a C, package: Package<T>) -> Self::Future<'a> {
        Box::pin(async move {
            let Some(encoder) = self.0.encoder_from_path(package.path().as_str()) else {
                return Err(Error::new(format!(
                    "Encoder not found for path: {}",
                    package.path()
                )));
            };

            let value: Bytes = encoder.save(package.content()).map_err(Error::new)?.into();

            Ok(package.map(|_| async move { value }).await)
        })
    }
}
