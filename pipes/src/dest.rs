use std::task::Poll;

use futures::{future::BoxFuture, ready, Future, TryFuture};
use pin_project_lite::pin_project;

use crate::Error;

pub trait Dest<T> {
    type Future<'a>: Future<Output = Result<(), Error>>
    where
        Self: 'a;

    fn call<'a>(&self, req: T) -> Self::Future<'a>;
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
{
    type Future<'a> = DestFnFuture<U>;
    fn call<'a>(&self, package: R) -> Self::Future<'a> {
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
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
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
    fn call<'a>(&self, args: T) -> Self::Future<'a> {
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
    fn call<'a>(&self, args: T) -> Self::Future<'a> {
        let sx = self.clone();
        Box::pin(async move {
            sx.send(args)
                .await
                .map_err(|_| Error::new("channel closed"))
        })
    }
}
