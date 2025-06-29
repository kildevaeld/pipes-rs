use crate::{Source, Work};
use alloc::boxed::Box;
use async_stream::try_stream;
use futures::TryStreamExt;
use heather::{HBoxStream, HSend, HSendSync};

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
    S: Source<C> + HSend,
    for<'a> S: 'a,
    S::Error: HSend,
    for<'a> S::Stream<'a>: HSend,
    S::Item: Clone + HSend,
    T1::Output: HSend,
    T1: Work<C, S::Item> + Clone + HSend,
    for<'a> T1: 'a,
    T1::Error: Into<S::Error> + HSend,
    for<'a> T1::Future<'a>: HSend,
    T2: Work<C, S::Item, Output = T1::Output> + Clone + HSend,
    for<'a> T2: 'a,
    T2::Error: Into<S::Error> + HSend,
    for<'a> T2::Future<'a>: HSend,
    for<'a> C: HSendSync + 'a,
{
    type Item = T1::Output;
    type Error = S::Error;
    type Stream<'a>
        = HBoxStream<'a, Result<Self::Item, Self::Error>>
    where
        S: 'a,
        C: 'a,
        T1: 'a,
        T2: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        Box::pin(try_stream! {
            let stream = self.source.create_stream(ctx);
            futures::pin_mut!(stream);

            while let Some(item) = stream.try_next().await? {

                yield self.work1.call(ctx, item.clone()).await?;
                yield self.work2.call(ctx, item).await?;

            }
        })
    }
}
