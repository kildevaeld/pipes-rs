use core::{mem::transmute, task::Poll};

use futures::{ready, Future, Stream};
use pin_project_lite::pin_project;
use bycat::{then::Then, Work};

use crate::Source;

impl<T1, T2, C> Source<C> for Then<T1, T2>
where
    T1: Source<C> + 'static,
    T2: Work<C, Result<T1::Item, T1::Error>> + 'static + Clone,
    C: Clone,
{
    type Item = T2::Output;
    type Error = T2::Error;

    type Stream<'a>
        = ThenStream<'a, T1, T2, C>
    where
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        ThenStream {
            stream: self.left.create_stream(ctx),
            work: self.right,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    #[project(!Unpin)]
    pub struct ThenStream<'a, T: 'static, W: 'static , C> where W: Work<C,Result<T::Item, T::Error>>, T: Source<C> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future<'a>>,
        ctx: &'a C
    }
}

impl<'a, T: 'static, W: 'static, C> Stream for ThenStream<'a, T, W, C>
where
    W: Work<C, Result<T::Item, T::Error>>,
    T: Source<C>,
{
    type Item = Result<W::Output, W::Error>;
    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.poll(cx));
                this.future.set(None);
                break Some(item);
            } else if let Some(item) = ready!(this.stream.as_mut().poll_next(cx)) {
                this.future
                    .set(Some(unsafe { transmute(this.work.call(this.ctx, item)) }));
            } else {
                break None;
            }
        })
    }
}
