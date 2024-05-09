use core::task::Poll;

use futures::{ready, Stream, TryStream};
use pin_project_lite::pin_project;

use crate::{Context, Error, WorkFn};

pub trait WorkMany<T> {
    type Item;
    type Stream: Stream<Item = Result<Self::Item, Error>>;

    fn call(&self, ctx: Context, pkg: T) -> Self::Stream;
}

pub fn work_many_fn<T, R, U>(func: T) -> WorkFn<T>
where
    T: Fn(Context, R) -> U,
    U: TryStream,
    U::Error: Into<Error>,
{
    WorkFn(func)
}

impl<T, U, R> WorkMany<R> for WorkFn<T>
where
    T: Fn(Context, R) -> U,
    U: TryStream,
    U::Error: Into<Error>,
{
    type Item = U::Ok;
    type Stream = WorkFnStream<U>;
    fn call(&self, ctx: Context, package: R) -> Self::Stream {
        WorkFnStream {
            stream: (self.0)(ctx, package),
        }
    }
}

pin_project! {
  pub struct WorkFnStream<U> {
    #[pin]
    stream: U
  }
}

impl<U> Stream for WorkFnStream<U>
where
    U: TryStream,
    U::Error: Into<Error>,
{
    type Item = Result<U::Ok, Error>;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.stream.try_poll_next(cx)) {
            Some(Ok(ret)) => Poll::Ready(Some(Ok(ret))),
            Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
            None => Poll::Ready(None),
        }
    }
}
