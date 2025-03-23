use futures::future::BoxFuture;
use pipes::Work;
use std::sync::Arc;

use crate::{Body, Package};

pub struct Serde<T>(Arc<toback::Toback<T>>)
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize;

impl<T> Clone for Serde<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Serde<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    pub fn new() -> Serde<T> {
        Self(toback::Toback::new().into())
    }
}

impl<C, T> Work<C, Package<Body>> for Serde<T>
where
    T: serde::de::DeserializeOwned + serde::ser::Serialize,
{
    type Output = Package<T>;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, _ctx: C, mut package: Package<Body>) -> Self::Future<'a> {
        Box::pin(async move {
            let Some(encoder) = self.0.encoder_from_path(package.path().as_str()) else {
                return Err(pipes::Error::new(format!(
                    "Encoder not found for path: {}",
                    package.path()
                )));
            };

            let body = package.replace_content(Body::Empty).bytes().await?;
            let value = encoder.load(&body).map_err(pipes::Error::new)?;

            Ok(Package {
                name: package.name,
                mime: package.mime,
                content: value,
                meta: package.meta,
            })
        })
    }
}
