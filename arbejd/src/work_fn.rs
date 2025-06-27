use core::task::Poll;

use crate::Work;
use futures_core::{TryFuture, ready};
use pin_project_lite::pin_project;

pub fn work_fn<T, C, R, U>(func: T) -> WorkFn<T>
where
    T: Fn(C, R) -> U,
    U: TryFuture,
{
    WorkFn(func)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkFn<T>(pub(crate) T);

impl<T, U, C, R> Work<C, R> for WorkFn<T>
where
    T: Fn(C, R) -> U,
    U: TryFuture,
    C: Clone,
{
    type Output = U::Ok;
    type Error = U::Error;
    type Future<'a>
        = WorkFnFuture<U>
    where
        Self: 'a,
        C: 'a;
    fn call<'a>(&'a self, ctx: &'a C, package: R) -> Self::Future<'a> {
        WorkFnFuture {
            future: (self.0)(ctx.clone(), package),
        }
    }
}

pin_project! {
  pub struct WorkFnFuture<U> {
    #[pin]
    future: U
  }
}

impl<U> Future for WorkFnFuture<U>
where
    U: TryFuture,
{
    type Output = Result<U::Ok, U::Error>;
    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        match ready!(this.future.try_poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
