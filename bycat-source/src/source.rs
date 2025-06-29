#[cfg(feature = "alloc")]
use crate::cloned::AsyncCloned;
use crate::{Pipeline, SourceUnit};
use bycat::{pipe::And, then::Then, IntoResult, ResultIter, Work};
use core::{mem::transmute, task::Poll};
use either::Either;
use futures::{ready, stream::TryFlatten, Stream, TryFuture, TryStream, TryStreamExt};
use pin_project_lite::pin_project;

pub trait Source<C> {
    type Item;
    type Error;
    type Stream<'a>: Stream<Item = Result<Self::Item, Self::Error>>
    where
        Self: 'a,
        C: 'a;
    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a>;
}

#[cfg(feature = "alloc")]
impl<T: 'static, E: 'static, C> Source<C> for alloc::vec::Vec<Result<T, E>> {
    type Item = T;
    type Error = E;
    type Stream<'a>
        = futures::stream::Iter<alloc::vec::IntoIter<Result<T, E>>>
    where
        Self: 'a,
        C: 'a;
    fn create_stream<'a>(self, _ctx: &'a C) -> Self::Stream<'a> {
        futures::stream::iter(self)
    }
}

pub fn iter<T>(iter: T) -> Iter<T> {
    Iter(iter)
}
pub struct Iter<T>(T);

impl<T, C> Source<C> for Iter<T>
where
    T: IntoIterator + 'static,
    T::Item: IntoResult,
{
    type Error = <T::Item as IntoResult>::Error;
    type Item = <T::Item as IntoResult>::Output;

    type Stream<'a>
        = futures::stream::Iter<ResultIter<T::IntoIter>>
    where
        C: 'a;

    fn create_stream<'a>(self, _ctx: &'a C) -> Self::Stream<'a> {
        futures::stream::iter(ResultIter(self.0.into_iter()))
    }
}

impl<T: 'static, E: 'static, C> Source<C> for Result<T, E> {
    type Item = T;
    type Error = E;

    type Stream<'a>
        = futures::stream::Once<futures::future::Ready<Result<T, E>>>
    where
        Self: 'a,
        C: 'a;

    fn create_stream<'a>(self, _ctx: &'a C) -> Self::Stream<'a> {
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
        Self::Item: TryStream<Error = Self::Error>,
    {
        Flatten { source: self }
    }

    #[cfg(feature = "alloc")]
    fn cloned<T1, T2>(self, work1: T1, work2: T2) -> AsyncCloned<Self, T1, T2>
    where
        Self: Sized,
    {
        AsyncCloned::new(self, work1, work2)
    }

    fn then<W>(self, work: W) -> Then<Self, W>
    where
        Self: Sized,
        W: Work<C, Result<Self::Item, Self::Error>>,
    {
        Then::new(self, work)
    }

    fn unit(self) -> SourceUnit<Self>
    where
        Self: Sized,
    {
        SourceUnit::new(self)
    }
}

impl<T, C> SourceExt<C> for T where T: Source<C> {}

impl<T1, T2, C> Source<C> for Either<T1, T2>
where
    T1: Source<C>,
    T2: Source<C>,
{
    type Item = Either<T1::Item, T2::Item>;

    type Error = Either<T1::Error, T2::Error>;

    type Stream<'a>
        = EitherSourceStream<'a, T1, T2, C>
    where
        T1: 'a,
        T2: 'a,
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        match self {
            Self::Left(left) => EitherSourceStream::T1 {
                stream: left.create_stream(ctx),
            },
            Self::Right(left) => EitherSourceStream::T2 {
                stream: left.create_stream(ctx),
            },
        }
    }
}

pin_project! {
    #[project = EitherStreamProj]
    pub enum EitherSourceStream<'a, T1: 'a, T2: 'a, C: 'a> where T1: Source<C>, T2: Source<C> {
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
    type Item = Result<Either<T1::Item, T2::Item>, Either<T1::Error, T2::Error>>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this {
            EitherStreamProj::T1 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Left(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(Either::Left(err)))),
                None => Poll::Ready(None),
            },
            EitherStreamProj::T2 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Right(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(Either::Right(err)))),
                None => Poll::Ready(None),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    W: Work<C, T::Item, Output = Option<T::Item>, Error = T::Error>,
{
    type Item = T::Item;

    type Error = T::Error;

    type Stream<'a>
        = FilterStream<'a, T, W, C>
    where
        T: 'a,
        W: 'a,
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        FilterStream {
            stream: self.source.create_stream(ctx),
            work: self.work,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    #[project(!Unpin)]
    pub struct FilterStream<'a, T: 'a, W: 'a, C: 'a> where T: Source<C>, W: Work<C,T::Item, Output = Option<T::Item>> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future<'a>>,
        ctx: &'a C
    }
}

impl<'a, T, W, C> Stream for FilterStream<'a, T, W, C>
where
    W: Work<C, T::Item, Output = Option<T::Item>, Error = T::Error>,
    T: Source<C>,
{
    type Item = Result<T::Item, T::Error>;
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
                    .set(Some(unsafe { transmute(this.work.call(this.ctx, item)) }));
            } else {
                break None;
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flatten<S> {
    source: S,
}

impl<S, C> Source<C> for Flatten<S>
where
    S: Source<C>,
    S::Item: TryStream<Error = S::Error>,
{
    type Item = <S::Item as TryStream>::Ok;

    type Error = <S::Item as TryStream>::Error;

    type Stream<'a>
        = TryFlatten<S::Stream<'a>>
    where
        S: 'a,
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        self.source.create_stream(ctx).try_flatten()
    }
}

pub fn stream<T>(stream: T) -> StreamSource<T>
where
    T: Stream + 'static,
    T::Item: IntoResult,
{
    StreamSource(stream)
}

pub struct StreamSource<T>(T);

impl<T, C> Source<C> for StreamSource<T>
where
    T: Stream + 'static,
    T::Item: IntoResult,
{
    type Item = <T::Item as IntoResult>::Output;

    type Error = <T::Item as IntoResult>::Error;

    type Stream<'a>
        = StreamSourceStream<T>
    where
        Self: 'a,
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        StreamSourceStream { stream: self.0 }
    }
}

pin_project! {
    pub struct StreamSourceStream<T>{
        #[pin]
        stream: T
    }
}

impl<T> Stream for StreamSourceStream<T>
where
    T: Stream,
    T::Item: IntoResult,
{
    type Item = Result<<T::Item as IntoResult>::Output, <T::Item as IntoResult>::Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.stream.poll_next(cx)) {
            Some(item) => Poll::Ready(Some(item.into_result())),
            None => Poll::Ready(None),
        }
    }
}
