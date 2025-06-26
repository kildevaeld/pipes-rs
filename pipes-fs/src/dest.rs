use std::path::PathBuf;

use futures::future::BoxFuture;
use mime::Mime;
use pipes::{Error, Work};
use pipes_package::{IntoPackage, Package};
use tokio::io::AsyncWriteExt;

use crate::Body;

#[derive(Debug, Clone)]
pub struct FsDest {
    path: std::path::PathBuf,
}

impl FsDest {
    pub fn new(path: impl Into<PathBuf>) -> FsDest {
        FsDest { path: path.into() }
    }
}

impl<C, T: IntoPackage<Body> + Send> Work<C, T> for FsDest
where
    T::Future: Send,
    for<'a> T: 'a,
{
    type Output = Package<Body>;

    type Future<'a> = BoxFuture<'a, Result<Package<Body>, Error>>;

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

pub trait Filter: Send + Sync {
    fn append(&self, pkg: &Package<Body>) -> bool;
}

impl Filter for Mime {
    fn append(&self, pkg: &Package<Body>) -> bool {
        pkg.mime() == self
    }
}

pub struct KravlDestination {
    root: PathBuf,
    append: Vec<Box<dyn Filter>>,
}

impl KravlDestination {
    pub fn new(path: impl Into<PathBuf>) -> KravlDestination {
        KravlDestination {
            root: path.into(),
            append: Default::default(),
        }
    }

    pub fn append_when<T>(mut self, filter: T) -> Self
    where
        T: Filter + 'static,
    {
        self.append.push(Box::new(filter));
        self
    }
}

impl KravlDestination {
    fn append(&self, pkg: &Package<Body>) -> bool {
        for filter in &self.append {
            if filter.append(pkg) {
                return true;
            }
        }
        false
    }
}

impl<C> Work<C, Package<Body>> for KravlDestination {
    type Output = Package<Body>;
    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, _ctx: C, mut req: Package<Body>) -> Self::Future<'a> {
        Box::pin(async move {
            let path = req.path().to_logical_path(&self.root);

            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            if self.append(&req) {
                let mut file = tokio::fs::OpenOptions::default()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .await
                    .map_err(pipes::Error::new)?;
                let bytes = req.replace_content(Body::Empty).bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
                file.write_all(b"\n").await.map_err(pipes::Error::new)?;
            } else {
                let mut file = tokio::fs::OpenOptions::default()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&path)
                    .await
                    .map_err(pipes::Error::new)?;
                let bytes = req.replace_content(Body::Empty).bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
            }

            Ok(req)
        })
    }
}
