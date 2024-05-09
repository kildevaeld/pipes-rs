use core::{marker::PhantomData, task::Poll};

use async_stream::try_stream;
use futures::{
    ready,
    stream::{BoxStream, FuturesUnordered},
    Stream, StreamExt, TryFuture, TryStream,
};
use pin_project_lite::pin_project;

use crate::{
    and::And,
    context::Context,
    error::Error,
    source::Source,
    work::{NoopWork, Work},
    wrap::Wrap,
};

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

impl<S, C> Pipeline<S, NoopWork, C> {
    pub fn new(source: S) -> Pipeline<S, NoopWork, C> {
        Pipeline {
            source,
            work: NoopWork,
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
    #[cfg(feature = "tokio")]
    pub fn concurrent(self) -> ConcurrentPipeline<S, W> {
        ConcurrentPipeline {
            source: self.source,
            work: self.work,
        }
    }

    pub fn wrap<F, U>(self, func: F) -> Pipeline<S, Wrap<W, F, C>, C>
    where
        Self: Sized,
        S: Source<C>,
        F: Fn(C, S::Item, Self) -> U + Clone,
        U: TryFuture,
        U::Error: Into<Error>,
    {
        Pipeline {
            source: self.source,
            work: Wrap::new(self.work, func),
            ctx: PhantomData,
        }
    }
}

impl<S, W, C> Source<C> for Pipeline<S, W, C>
where
    S: Source<C>,
    W: Work<C, S::Item> + 'static,
    C: Clone,
    for<'a> C: 'a,
{
    type Item = W::Output;
    type Stream<'a> = PipelineStream<S::Stream<'a>, W, C, S::Item> where S: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        PipelineStream {
            stream: self.source.call(ctx.clone()),
            work: self.work,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    pub struct PipelineStream<T, W, C, R> where W: Work<C,R> {
        #[pin]
        stream: T,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: C
    }
}

impl<T, W, C, R> Stream for PipelineStream<T, W, C, R>
where
    W: Work<C, R>,
    T: Stream<Item = Result<R, Error>>,
    C: Clone,
{
    type Item = Result<W::Output, Error>;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                break Some(item);
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.future
                    .set(Some(this.work.call(this.ctx.clone(), item)));
            } else {
                break None;
            }
        })
    }
}

#[cfg(feature = "tokio")]
pub struct ConcurrentPipeline<S, W> {
    source: S,
    work: W,
}

#[cfg(feature = "tokio")]
impl<S, W, C> Source<C> for ConcurrentPipeline<S, W>
where
    S: Source<C> + Send + 'static,
    for<'a> S::Stream<'a>: Send,
    S::Item: Send,
    W: Work<C, S::Item> + Send + Sync + 'static,
    W::Output: Send,
    W::Future: Send,
    for<'a> C: Send + 'a,
    C: Clone,
{
    type Item = W::Output;

    type Stream<'a> = BoxStream<'a, Result<Self::Item, Error>>;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        alloc::boxed::Box::pin(try_stream! {

            let stream = self.source.call(ctx.clone());
            futures::pin_mut!(stream);
            let mut queue = FuturesUnordered::new();

            loop {
                tokio::select! {
                    Some(next) = stream.next() => {
                        let ctx = ctx.clone();
                        queue.push(async  {
                            let next = next?;
                            self.work.call(ctx, next).await
                        });

                    }
                    Some(next) = queue.next() => {
                        yield next?;
                    }
                    else => break
                }

            }
        })
    }
}
