use crate::work::Work;
use core::{marker::PhantomData, task::Poll};
use futures_core::ready;
use pin_project_lite::pin_project;

pub struct MapErr<W, T, E> {
    work: W,
    map: T,
    error: PhantomData<fn() -> E>,
}

impl<W: Clone, T: Clone, E> Clone for MapErr<W, T, E> {
    fn clone(&self) -> Self {
        MapErr {
            work: self.work.clone(),
            map: self.map.clone(),
            error: PhantomData,
        }
    }
}

impl<W: Copy, T: Copy, E> Copy for MapErr<W, T, E> {}

unsafe impl<W: Send, T: Send, E> Send for MapErr<W, T, E> {}

unsafe impl<W: Sync, T: Sync, E> Sync for MapErr<W, T, E> {}

impl<W, T, E> MapErr<W, T, E> {
    pub fn new(work: W, map: T) -> MapErr<W, T, E> {
        MapErr {
            work,
            map,
            error: PhantomData,
        }
    }
}

impl<W, T, E, C, I> Work<C, I> for MapErr<W, T, E>
where
    W: Work<C, I>,
    T: Fn(W::Error) -> E,
{
    type Output = W::Output;

    type Error = E;

    type Future<'a>
        = MapErrFuture<'a, W, C, I, T, E>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: I) -> Self::Future<'a> {
        MapErrFuture {
            future: self.work.call(context, req),
            map: &self.map,
            error: PhantomData,
        }
    }
}

pin_project! {
  pub struct MapErrFuture<'a, W: 'a, C: 'a, I, T, E>
  where
    W: Work<C, I>
   {
    #[pin]
    future: W::Future<'a>,
    map: &'a T,
    error: PhantomData<E>
  }
}

impl<'a, W, C, I, T, E> Future for MapErrFuture<'a, W, C, I, T, E>
where
    W: Work<C, I> + 'a,
    T: Fn(W::Error) -> E,
{
    type Output = Result<W::Output, E>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        match ready!(this.future.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err((this.map)(err))),
        }
    }
}
