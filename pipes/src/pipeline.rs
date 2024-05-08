use core::task::Poll;

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
};

pub struct Pipeline<S, W> {
    source: S,
    work: W,
}

impl<S> Pipeline<S, NoopWork> {
    pub fn new(source: S) -> Pipeline<S, NoopWork> {
        Pipeline {
            source,
            work: NoopWork,
        }
    }
}

impl<S, W> Pipeline<S, W> {
    pub fn pipe<T>(self, work: T) -> Pipeline<S, And<W, T>> {
        Pipeline {
            source: self.source,
            work: And::new(self.work, work),
        }
    }

    #[cfg(feature = "tokio")]
    pub fn concurrent(self) -> ConcurrentPipeline<S, W> {
        ConcurrentPipeline {
            source: self.source,
            work: self.work,
        }
    }
}

impl<S, W> Source for Pipeline<S, W>
where
    S: Source,
    W: Work<S::Item>,
{
    type Item = W::Output;
    type Stream = PipelineStream<S::Stream, W, S::Item>;

    fn call(self) -> Self::Stream {
        PipelineStream {
            stream: self.source.call(),
            work: self.work,
            future: None,
            ctx: Context::new(),
        }
    }
}

pin_project! {
    pub struct PipelineStream<T, W, R> where W: Work<R> {
        #[pin]
        stream: T,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: Context
    }
}

impl<T, W, R> Stream for PipelineStream<T, W, R>
where
    W: Work<R>,
    T: Stream<Item = Result<R, Error>>,
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
impl<S, W> Source for ConcurrentPipeline<S, W>
where
    S: Source + Send + 'static,
    S::Stream: Send,
    S::Item: Send,
    W: Work<S::Item> + Send + Sync + 'static,
    W::Output: Send,
    W::Future: Send,
{
    type Item = W::Output;

    type Stream = BoxStream<'static, Result<Self::Item, Error>>;

    fn call(self) -> Self::Stream {
        alloc::boxed::Box::pin(try_stream! {

            let stream = self.source.call();
            futures::pin_mut!(stream);
            let mut queue = FuturesUnordered::new();
            loop {
                tokio::select! {
                    Some(next) = stream.next() => {
                        queue.push(async  {
                            let next = next?;
                            self.work.call(Context {  }, next).await
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
