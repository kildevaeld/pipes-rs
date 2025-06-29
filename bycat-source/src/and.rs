use crate::{source::Source, Unit};
use bycat::pipe::And;
use core::task::Poll;
use futures::{ready, Future};
use pin_project_lite::pin_project;

impl<T1, T2, C> Source<C> for And<T1, T2>
where
    T1: Source<C>,
    T2: Source<C, Item = T1::Item, Error = T1::Error>,
{
    type Item = T1::Item;
    type Error = T1::Error;
    type Stream<'a>
        = futures::stream::Select<T1::Stream<'a>, T2::Stream<'a>>
    where
        T1: 'a,
        T2: 'a,
        C: 'a;
    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        futures::stream::select(self.left.create_stream(ctx), self.right.create_stream(ctx))
    }
}

impl<T1, T2, C> Unit<C> for And<T1, T2>
where
    T1: Unit<C>,
    T2: Unit<C>,
{
    type Future<'a>
        = AndUnitFuture<T1::Future<'a>, T2::Future<'a>>
    where
        T1: 'a,
        T2: 'a,
        C: 'a;

    fn run<'a>(self, ctx: &'a C) -> Self::Future<'a> {
        AndUnitFuture {
            future: futures::future::join(self.left.run(ctx), self.right.run(ctx)),
        }
    }
}

pin_project! {
    pub struct AndUnitFuture<T1, T2> where T1: Future<Output = ()>, T2: Future<Output = ()> {
        #[pin]
        future: futures::future::Join<T1, T2>
    }
}

impl<T1, T2> Future for AndUnitFuture<T1, T2>
where
    T1: Future<Output = ()>,
    T2: Future<Output = ()>,
{
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.future.poll(cx));
        Poll::Ready(())
    }
}
