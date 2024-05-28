use core::task::Poll;
use std::path::PathBuf;

use alloc::boxed::Box;
#[cfg(feature = "tokio")]
use futures::future::BoxFuture;
use futures::{ready, Future, TryFuture};
use pin_project_lite::pin_project;

use crate::{Error, IntoPackage, Package, Work};

pub trait Dest<T> {
    type Future<'a>: Future<Output = Result<(), Error>>
    where
        Self: 'a,
        T: 'a;

    fn call<'a>(&'a self, req: T) -> Self::Future<'a>;
}

pub fn dest_fn<T>(func: T) -> DestFn<T> {
    DestFn(func)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DestFn<T>(T);

impl<T, U, R> Dest<R> for DestFn<T>
where
    T: Fn(R) -> U + 'static,
    U: TryFuture<Ok = ()>,
    U::Error: Into<Error>,
    for<'a> R: 'a,
{
    type Future<'a> = DestFnFuture<U>;
    fn call<'a>(&'a self, package: R) -> Self::Future<'a> {
        DestFnFuture {
            future: (self.0)(package),
        }
    }
}

pin_project! {
  pub struct DestFnFuture<U> {
    #[pin]
    future: U
  }
}

impl<U> Future for DestFnFuture<U>
where
    U: TryFuture,
    U::Error: Into<Error>,
{
    type Output = Result<U::Ok, Error>;
    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        match ready!(this.future.try_poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(err.into())),
        }
    }
}

#[cfg(feature = "tokio")]
impl<T: Send + 'static> Dest<T> for tokio::sync::mpsc::Sender<T> {
    type Future<'a> = BoxFuture<'a, Result<(), Error>>;
    fn call<'a>(&'a self, args: T) -> Self::Future<'a> {
        let sx = self.clone();
        Box::pin(async move {
            sx.send(args)
                .await
                .map_err(|_| Error::new("channel closed"))
        })
    }
}

#[cfg(feature = "async-channel")]
impl<T: Send + 'static> Dest<T> for async_channel::Sender<T> {
    type Future<'a> = BoxFuture<'a, Result<(), Error>>;
    fn call<'a>(&'a self, args: T) -> Self::Future<'a> {
        let sx = self.clone();
        Box::pin(async move {
            sx.send(args)
                .await
                .map_err(|_| Error::new("channel closed"))
        })
    }
}

#[cfg(feature = "tokio")]
#[derive(Debug, Clone)]
pub struct FsDest {
    path: std::path::PathBuf,
}

#[cfg(feature = "tokio")]
impl FsDest {
    pub fn new(path: impl Into<PathBuf>) -> FsDest {
        FsDest { path: path.into() }
    }
}

#[cfg(feature = "tokio")]
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

#[cfg(feature = "tokio")]
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
