use std::path::PathBuf;

use futures::future::BoxFuture;
use pipes::{fs::File, Dest, Error, IntoPackage, Package, Work};
use relative_path::{RelativePath, RelativePathBuf};

#[derive(Debug, Clone)]
pub struct Fs {
    root: PathBuf,
}

impl Fs {
    pub fn new(path: PathBuf) -> Fs {
        Fs { root: path }
    }

    pub async fn get(&self, path: impl AsRef<RelativePath>) -> Result<Package, pipes::Error> {
        let full_path = path.as_ref().to_logical_path(&self.root);
        let mut pkg = File::from_path(&full_path).await?.into_package().await?;
        pkg.set_path(path.as_ref().to_relative_path_buf());
        Ok(pkg)
    }

    pub async fn write(&self, package: &mut Package) -> Result<(), Error> {
        package.write_to(&self.root).await.map_err(Error::new)?;
        Ok(())
    }
}

impl<C> Work<C, Package> for Fs {
    type Output = Package;

    type Future<'a> = BoxFuture<'a, Result<Self::Output, Error>>;

    fn call<'a>(&'a self, _ctx: C, mut package: Package) -> Self::Future<'a> {
        Box::pin(async move {
            self.write(&mut package).await?;
            Ok(package)
        })
    }
}

impl<C> Work<C, RelativePathBuf> for Fs {
    type Output = Package;

    type Future<'a> = BoxFuture<'a, Result<Self::Output, Error>>;

    fn call<'a>(&'a self, _ctx: C, path: RelativePathBuf) -> Self::Future<'a> {
        Box::pin(async move { self.get(path).await })
    }
}

impl Dest<Package> for Fs {
    fn call<'a>(&'a self, mut req: Package) -> Self::Future<'a> {
        Box::pin(async move { self.write(&mut req).await })
    }

    type Future<'a> = BoxFuture<'a, Result<(), Error>>;
}

#[derive(Debug, Clone)]
pub struct Cached<T> {
    work: T,
    root: PathBuf,
}

impl<T> Cached<T> {
    pub fn new(root: PathBuf, work: T) -> Cached<T> {
        Cached { work, root }
    }
}

impl<W, C> Work<C, Package> for Cached<W>
where
    C: Send + 'static,
    W: Work<C, Package, Output = Package> + Send + Sync + 'static,
    for<'a> W::Future<'a>: Send,
{
    type Output = Package;

    type Future<'a> = BoxFuture<'a, Result<Self::Output, Error>>;
    fn call<'a>(&'a self, ctx: C, package: Package) -> Self::Future<'a> {
        Box::pin(async move {
            let cache_path = package.path().to_logical_path(&self.root);

            if tokio::fs::try_exists(&cache_path)
                .await
                .map_err(Error::new)?
            {
                let pkg = File::from_path(&cache_path).await?.into_package().await?;
                Ok(pkg)
            } else {
                let mut pkg = self.work.call(ctx, package).await?;
                pkg.write_to(&cache_path).await?;

                Ok(pkg)
            }
        })
    }
}
