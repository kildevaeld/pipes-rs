use core::{future::ready, task::Poll};

use alloc::{string::ToString, sync::Arc};
use either::Either;
use futures::{
    pin_mut, ready, stream::TryFlatten, Stream, StreamExt, TryFuture, TryStream, TryStreamExt,
};
use pin_project_lite::pin_project;

use crate::{
    and::And, cloned::AsyncCloned, dest::Dest, error::Error, then::Then, work_fn, Context,
    Pipeline, SourceUnit, Unit, Work,
};

pub trait Source<C> {
    type Item;
    type Stream<'a>: Stream<Item = Result<Self::Item, Error>>
    where
        Self: 'a;
    fn call<'a>(self, ctx: C) -> Self::Stream<'a>;
}

impl<T: 'static, C> Source<C> for alloc::vec::Vec<Result<T, Error>> {
    type Item = T;
    type Stream<'a> = futures::stream::Iter<alloc::vec::IntoIter<Result<T, Error>>>;
    fn call<'a>(self, _ctx: C) -> Self::Stream<'a> {
        futures::stream::iter(self)
    }
}

impl<T: 'static, C> Source<C> for Result<T, Error> {
    type Item = T;

    type Stream<'a> = futures::stream::Once<futures::future::Ready<Result<T, Error>>>;

    fn call<'a>(self, _ctx: C) -> Self::Stream<'a> {
        futures::stream::once(futures::future::ready(self))
    }
}

pub trait SourceExt<C>: Source<C> {
    fn and<S>(self, source: S) -> And<Self, S>
    where
        Self: Sized,
        S: Source<C>,
    {
        And::new(self, source)
    }

    fn dest<T>(self, dest: T) -> SourceUnit<Self, T>
    where
        Self: Sized,
        T: Dest<Self::Item>,
    {
        SourceUnit::new(self, dest)
    }

    fn filter<W>(self, work: W) -> Filter<Self, W>
    where
        Self: Sized,
        W: Work<C, Self::Item, Output = Option<Self::Item>>,
    {
        Filter::new(self, work)
    }

    fn pipe<W>(self, work: W) -> Pipeline<Self, W, C>
    where
        Self: Sized,
        W: Work<C, Self::Item>,
    {
        Pipeline::new_with(self, work)
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: TryStream<Error = Error>,
    {
        Flatten { source: self }
    }

    fn cloned<T1, T2>(self, work1: T1, work2: T2) -> AsyncCloned<Self, T1, T2>
    where
        Self: Sized,
    {
        AsyncCloned::new(self, work1, work2)
    }

    fn then<W>(self, work: W) -> Then<Self, W>
    where
        Self: Sized,
        W: Work<C, Result<Self::Item, Error>>,
    {
        Then::new(self, work)
    }

    #[cfg(feature = "tokio")]
    fn spawn(self) -> SpawnSource<Self, C>
    where
        Self: Sized + Send + 'static,
        Self::Item: Send + 'static,
        for<'a> Self::Stream<'a>: Send,
        for<'a> C: Send + 'a,
    {
        SpawnSource::new(self)
    }
}

impl<T, C> SourceExt<C> for T where T: Source<C> {}

#[cfg(feature = "async-channel")]
impl<T> Source for async_channel::Receiver<T> {
    type Item = T;
    type Stream = AsyncChannelStream<T>;

    fn call(self) -> Self::Stream {
        AsyncChannelStream { rx: self }
    }
}

#[cfg(feature = "async-channel")]
pin_project! {
    pub struct AsyncChannelStream<T> {
        #[pin]
        rx: async_channel::Receiver<T>,
    }
}

#[cfg(feature = "async-channel")]
impl<T> Stream for AsyncChannelStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        Poll::Ready(ready!(this.rx.poll_next(cx)).map(Ok))
    }
}

#[cfg(feature = "tokio")]
impl<T: 'static, C> Source<C> for tokio::sync::mpsc::Receiver<Result<T, Error>> {
    type Item = T;
    type Stream<'a> = TokioChannelStream<T>;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        TokioChannelStream { rx: self }
    }
}

#[cfg(feature = "tokio")]
pin_project! {
    pub struct TokioChannelStream<T> {
        #[pin]
        rx: tokio::sync::mpsc::Receiver<Result<T, Error>>,
    }
}

#[cfg(feature = "tokio")]
impl<T> Stream for TokioChannelStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(ready!(this.rx.poll_recv(cx)))
    }
}

#[cfg(feature = "tokio")]
pub struct SpawnSource<S, C>
where
    S: Source<C>,
{
    rx: tokio::sync::mpsc::Receiver<Result<S::Item, Error>>,
    start: futures::channel::oneshot::Sender<C>,
}

#[cfg(feature = "tokio")]
impl<S, C> SpawnSource<S, C>
where
    S: Source<C> + Send + 'static,
    S::Item: Send + 'static,
    for<'a> S::Stream<'a>: Send,
    C: Send + 'static,
{
    pub fn new(source: S) -> SpawnSource<S, C> {
        let (sx, rx) = tokio::sync::mpsc::channel(10);
        let (start, wait) = futures::channel::oneshot::channel();
        tokio::spawn(async move {
            let Ok(ctx) = wait.await else { return };

            let stream = source.call(ctx);
            pin_mut!(stream);
            while let Some(next) = stream.next().await {
                sx.send(next).await.ok();
            }
        });

        SpawnSource { rx, start }
    }
}

#[cfg(feature = "tokio")]
impl<S: 'static, C> Source<C> for SpawnSource<S, C>
where
    S: Source<C>,
    C: 'static,
{
    type Item = S::Item;

    type Stream<'a> = TokioChannelStream<S::Item>;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        self.start.send(ctx).ok();
        TokioChannelStream { rx: self.rx }
    }
}

impl<T1, T2, C> Source<C> for Either<T1, T2>
where
    T1: Source<C>,
    T2: Source<C>,
{
    type Item = Either<T1::Item, T2::Item>;

    type Stream<'a> = EitherSourceStream<'a, T1, T2, C> where T1: 'a, T2: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        match self {
            Self::Left(left) => EitherSourceStream::T1 {
                stream: left.call(ctx),
            },
            Self::Right(left) => EitherSourceStream::T2 {
                stream: left.call(ctx),
            },
        }
    }
}

pin_project! {
    #[project = EitherStreamProj]
    pub enum EitherSourceStream<'a, T1: 'a, T2: 'a, C> where T1: Source<C>, T2: Source<C> {
        T1 {
            #[pin]
            stream: T1::Stream<'a>
        },
        T2 {
            #[pin]
            stream: T2::Stream<'a>
        }
    }
}

impl<'a, T1, T2, C> Stream for EitherSourceStream<'a, T1, T2, C>
where
    T1: Source<C>,
    T2: Source<C>,
{
    type Item = Result<Either<T1::Item, T2::Item>, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this {
            EitherStreamProj::T1 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Left(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(err))),
                None => Poll::Ready(None),
            },
            EitherStreamProj::T2 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Right(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(err))),
                None => Poll::Ready(None),
            },
        }
    }
}

pub struct Filter<T, W> {
    source: T,
    work: W,
}

impl<T, W> Filter<T, W> {
    pub fn new(source: T, work: W) -> Filter<T, W> {
        Filter { source, work }
    }
}

impl<T, W: 'static, C> Source<C> for Filter<T, W>
where
    T: Source<C>,
    W: Work<C, T::Item, Output = Option<T::Item>>,
    C: Clone,
{
    type Item = T::Item;

    type Stream<'a> = FilterStream<'a, T, W, C> where T:'a, W: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        FilterStream {
            stream: self.source.call(ctx.clone()),
            work: self.work,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    pub struct FilterStream<'a, T: 'a, W, C> where T: Source<C>, W: Work<C,T::Item, Output = Option<T::Item>> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: C
    }
}

impl<'a, T, W, C> Stream for FilterStream<'a, T, W, C>
where
    W: Work<C, T::Item, Output = Option<T::Item>>,
    T: Source<C>,
    C: Clone,
{
    type Item = Result<T::Item, Error>;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                match item {
                    Ok(Some(ret)) => break Some(Ok(ret)),
                    Err(err) => break Some(Err(err)),
                    _ => {}
                }
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.future
                    .set(Some(this.work.call(this.ctx.clone(), item)));
            } else {
                break None;
            }
        })
    }
}

pub struct Flatten<S> {
    source: S,
}

impl<S, C> Source<C> for Flatten<S>
where
    S: Source<C>,
    S::Item: TryStream<Error = Error>,
{
    type Item = <S::Item as TryStream>::Ok;

    type Stream<'a> = TryFlatten<S::Stream<'a>> where S: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        self.source.call(ctx).try_flatten()
    }
}

#[cfg(feature = "tokio")]
pub struct Producer<T> {
    sx: tokio::sync::mpsc::Sender<Result<T, Error>>,
}

impl<T> Clone for Producer<T> {
    fn clone(&self) -> Self {
        Producer {
            sx: self.sx.clone(),
        }
    }
}

#[cfg(feature = "tokio")]
impl<T> Producer<T> {
    pub fn new() -> (Producer<T>, tokio::sync::mpsc::Receiver<Result<T, Error>>) {
        let (sx, rx) = tokio::sync::mpsc::channel(10);
        (Producer { sx }, rx)
    }

    pub async fn send(&self, value: T) -> Result<(), Error> {
        self.sx
            .send(Ok(value))
            .await
            .map_err(|_| Error::new("channel closed"))
    }
}
