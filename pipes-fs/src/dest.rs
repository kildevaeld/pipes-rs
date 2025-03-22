use std::path::PathBuf;

use futures::future::BoxFuture;
use pipes::{Dest, Error, Work};

use crate::{IntoPackage, Package};

#[derive(Debug, Clone)]
pub struct FsDest {
    path: std::path::PathBuf,
}

impl FsDest {
    pub fn new(path: impl Into<PathBuf>) -> FsDest {
        FsDest { path: path.into() }
    }
}

impl<T: IntoPackage + Send> Dest<T> for FsDest
where
    T::Future: Send,
    for<'a> T: 'a,
{
    type Future<'a> = BoxFuture<'a, Result<(), Error>>;

    fn call<'a>(&'a self, req: T) -> Self::Future<'a> {
        Box::pin(async move {
            //
            if !tokio::fs::try_exists(&self.path)
                .await
                .map_err(Error::new)?
            {
                tokio::fs::create_dir_all(&self.path)
                    .await
                    .map_err(Error::new)?
            }
            req.into_package().await?.write_to(&self.path).await
        })
    }
}

impl<C, T: IntoPackage + Send> Work<C, T> for FsDest
where
    T::Future: Send,
    for<'a> T: 'a,
{
    type Output = Package;

    type Future<'a> = BoxFuture<'a, Result<Package, Error>>;

    fn call<'a>(&'a self, _ctx: C, req: T) -> Self::Future<'a> {
        let path = self.path.clone();
        Box::pin(async move {
            if !tokio::fs::try_exists(&path).await.map_err(Error::new)? {
                tokio::fs::create_dir_all(&path).await.map_err(Error::new)?
            }

            let mut package = req.into_package().await?;
            package.write_to(&path).await?;

            Ok(package)
        })
    }
}
