use core::task::Poll;
use futures::{ready, Future, TryFuture};
use pin_project_lite::pin_project;

use crate::Error;

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
