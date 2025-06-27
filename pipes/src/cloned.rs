use alloc::boxed::Box;
use async_stream::try_stream;
use futures::{stream::BoxStream, Future, TryStreamExt};

use crate::{Error, Source, Work};

pub trait AsyncClone: Sized {
    type Future<'a>: Future<Output = Result<Self, Error>>
    where
        Self: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a>;
}

impl<T> AsyncClone for T
where
    T: Clone,
{
    type Future<'a>
        = futures::future::Ready<Result<Self, Error>>
    where
        T: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a> {
        futures::future::ready(Ok(self.clone()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AsyncCloned<S, T1, T2> {
    source: S,
    work1: T1,
    work2: T2,
}

impl<S, T1, T2> AsyncCloned<S, T1, T2> {
    pub fn new(source: S, work1: T1, work2: T2) -> AsyncCloned<S, T1, T2> {
        AsyncCloned {
            source,
            work1,
            work2,
        }
    }
}

impl<S, T1, T2, C> Source<C> for AsyncCloned<S, T1, T2>
where
    S: Source<C> + 'static + Send,
    for<'a> S::Stream<'a>: Send,
    S::Item: AsyncClone + Send,
    for<'a> <S::Item as AsyncClone>::Future<'a>: Send,
    T1::Output: Send,
    T1: Work<C, S::Item> + 'static + Clone + Send,
    for<'a> T1::Future<'a>: Send,
    T2: Work<C, S::Item, Output = T1::Output> + 'static + Clone + Send,
    for<'a> T2::Future<'a>: Send,
    for<'a> C: Send + 'a,
    C: Clone,
{
    type Item = T1::Output;

    type Stream<'a> = BoxStream<'a, Result<T1::Output, Error>>;

    fn create_stream<'a>(self, ctx: C) -> Self::Stream<'a> {
        Box::pin(try_stream! {
            let stream = self.source.create_stream(ctx.clone());
            futures::pin_mut!(stream);

            while let Some(mut item) = stream.try_next().await? {
                let clone = item.async_clone().await?;

                yield self.work1.call(ctx.clone(), clone).await?;
                yield self.work2.call(ctx.clone(), item).await?;

            }
        })
    }
}
