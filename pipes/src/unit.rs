use core::task::Poll;

use futures::{ready, Future, TryStream};
use pin_project_lite::pin_project;

use crate::{and::And, Source};

pub trait Unit<C> {
    type Future<'a>: Future<Output = ()>
    where
        Self: 'a;
    fn run<'a>(self, ctx: C) -> Self::Future<'a>;
}

pub trait UnitExt<C>: Unit<C> {
    fn and<T>(self, next: T) -> And<Self, T>
    where
        Self: Sized,
        T: Unit<C>,
    {
        And::new(self, next)
    }
}

impl<T, C> UnitExt<C> for T where T: Unit<C> {}

#[derive(Debug, Clone, Copy)]
pub struct SourceUnit<S> {
    source: S,
}

impl<S> SourceUnit<S> {
    pub fn new(source: S) -> SourceUnit<S> {
        SourceUnit { source }
    }
}

impl<S, C> Unit<C> for SourceUnit<S>
where
    S: Source<C> + 'static,
    for<'a> S::Item: 'a,
{
    type Future<'a> = SourceUnitFure<'a, S, C>;

    fn run<'a>(self, ctx: C) -> Self::Future<'a> {
        SourceUnitFure {
            stream: self.source.create_stream(ctx),
        }
    }
}

pin_project! {
    #[project(!Unpin)]
    pub struct SourceUnitFure<'a, S: 'a, C> where  S: Source<C>, S::Item: 'a {
        #[pin]
        stream: S::Stream<'a>,

    }
}

impl<'a, S, C> Future for SourceUnitFure<'a, S, C>
where
    S: Source<C> + 'a,
{
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let mut this = self.as_mut().project();

        Poll::Ready(loop {
            match ready!(this.stream.as_mut().try_poll_next(cx)) {
                None => break (),
                _ => continue,
            }
        })
    }
}
