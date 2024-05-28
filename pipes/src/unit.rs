use core::{mem::transmute, task::Poll};

use futures::{ready, Future, TryStream};
use pin_project_lite::pin_project;

use crate::{and::And, dest::Dest, Source};

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

pub struct SourceUnit<S, T> {
    source: S,
    dest: T,
}

impl<S, T> SourceUnit<S, T> {
    pub fn new(source: S, dest: T) -> SourceUnit<S, T> {
        SourceUnit { source, dest }
    }
}

impl<S, T, C> Unit<C> for SourceUnit<S, T>
where
    T: Dest<S::Item> + 'static,
    S: Source<C> + 'static,
    for<'a> S::Item: 'a,
{
    type Future<'a> = SourceUnitFure<'a, S, T, C>;

    fn run<'a>(self, ctx: C) -> Self::Future<'a> {
        SourceUnitFure {
            stream: self.source.call(ctx),
            dest: self.dest,
            future: None,
        }
    }
}

pin_project! {
    pub struct SourceUnitFure<'a, S: 'a, T:'a, C> where T: Dest<S::Item>, S: Source<C>, S::Item: 'a {
        #[pin]
        stream: S::Stream<'a>,
        dest: T,
        #[pin]
        future: Option<T::Future<'a>>
    }
}

impl<'a, S, T, C> Future for SourceUnitFure<'a, S, T, C>
where
    T: Dest<S::Item> + 'a,
    S: Source<C> + 'a,
{
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let mut this = self.as_mut().project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                // TODO: What to do about errors
                let _item = ready!(fut.poll(cx));
                this.future.set(None);
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)) {
                if let Ok(ok) = item {
                    this.future
                        .set(Some(unsafe { transmute(this.dest.call(ok)) }));
                }
            } else {
                break ();
            }
        })
    }
}
