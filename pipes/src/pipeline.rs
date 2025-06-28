use crate::source::Source;
use core::{marker::PhantomData, mem::transmute, task::Poll};
use futures::{ready, Stream, TryFuture, TryStream};
use pin_project_lite::pin_project;
use bycat::{NoopWork, Work};

#[derive(Debug)]
pub struct Pipeline<S, W, C> {
    source: S,
    work: W,
    ctx: PhantomData<C>,
}

impl<S: Clone, W: Clone, C> Clone for Pipeline<S, W, C> {
    fn clone(&self) -> Self {
        Pipeline {
            source: self.source.clone(),
            work: self.work.clone(),
            ctx: PhantomData,
        }
    }
}

impl<S: Copy, W: Copy, C> Copy for Pipeline<S, W, C> {}

unsafe impl<S: Send, W: Send, C> Send for Pipeline<S, W, C> {}

unsafe impl<S: Sync, W: Sync, C> Sync for Pipeline<S, W, C> {}

impl<S, C> Pipeline<S, NoopWork<S::Error>, C>
where
    S: Source<C>,
{
    pub fn new(source: S) -> Pipeline<S, NoopWork<S::Error>, C> {
        Pipeline {
            source,
            work: NoopWork::default(),
            ctx: PhantomData,
        }
    }
}

impl<S, W, C> Pipeline<S, W, C> {
    pub fn new_with(source: S, work: W) -> Pipeline<S, W, C> {
        Pipeline {
            source,
            work,
            ctx: PhantomData,
        }
    }
}

impl<S, W, C> Pipeline<S, W, C> {
    // pub fn wrap<F, U>(self, func: F) -> Pipeline<S, Wrap<W, F, C>, C>
    // where
    //     Self: Sized,
    //     S: Source<C>,
    //     F: Fn(C, S::Item, W) -> U + Clone,
    //     U: TryFuture,
    //     U::Error: Into<Error>,
    // {
    //     Pipeline {
    //         source: self.source,
    //         work: Wrap::new(self.work, func),
    //         ctx: PhantomData,
    //     }
    // }
}

impl<S, W, C> Source<C> for Pipeline<S, W, C>
where
    S: Source<C> + 'static,
    W: Work<C, S::Item, Error = S::Error>,
{
    type Item = W::Output;
    type Error = W::Error;
    type Stream<'a>
        = PipelineStream<'a, S, W, C>
    where
        C: 'a,
        W: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        PipelineStream {
            stream: self.source.create_stream(ctx),
            work: self.work,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    #[project(!Unpin)]
    pub struct PipelineStream<'a, T: 'a, W: 'a, C> where W: Work<C, T::Item>, T: Source<C> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future<'a>>,
        ctx: &'a C
    }
}

impl<'a, T: 'a, W: 'a, C> Stream for PipelineStream<'a, T, W, C>
where
    W: Work<C, T::Item, Error = T::Error>,
    T: Source<C>,
    Self: 'a,
{
    type Item = Result<W::Output, W::Error>;
    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                break Some(item);
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.future
                    .set(Some(unsafe { transmute(this.work.call(this.ctx, item)) }));
            } else {
                break None;
            }
        })
    }
}
