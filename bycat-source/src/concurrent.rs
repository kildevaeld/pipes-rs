use alloc::boxed::Box;
use bycat::Work;
use futures::{pin_mut, stream::FuturesUnordered, StreamExt};
use heather::{HBoxStream, HSend, HSendSync};

use crate::Source;

pub struct Concurrent<S, T> {
    source: S,
    work: T,
}

impl<S, T> Concurrent<S, T> {
    pub fn new(source: S, work: T) -> Concurrent<S, T> {
        Concurrent { source, work }
    }
}

impl<S, T, C> Source<C> for Concurrent<S, T>
where
    S: Source<C> + HSend,
    S::Item: HSend,
    S::Error: HSend,
    for<'a> S::Stream<'a>: HSend,
    T: Work<C, S::Item> + HSend,
    T::Output: HSend,
    T::Error: Into<S::Error> + HSend,
    for<'a> T::Future<'a>: HSend,
    C: HSendSync,
    for<'a> S: 'a,
    for<'a> T: 'a,
    for<'a> C: 'a,
{
    type Error = S::Error;
    type Item = T::Output;

    type Stream<'a>
        = HBoxStream<'a, Result<Self::Item, Self::Error>>
    where
        S: 'a,
        T: 'a,
        C: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        let stream = async_stream::stream! {

          let  stream = self.source.create_stream(ctx);
          pin_mut!(stream);

          let mut futures = FuturesUnordered::new();

          while let Some(next) = stream.next().await {
            match next {
              Ok(ret) => futures.push(self.work.call(ctx, ret)),
              Err(err) => {
                yield Err(err)
              }
            }

            if let Some(next) = futures.next().await {
              yield next.map_err(Into::into)
            }
          }

          while let Some(next) = futures.next().await {
            yield next.map_err(Into::into);
          }

        };

        Box::pin(stream)
    }
}
