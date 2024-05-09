use core::task::Poll;

use futures::{ready, Future, TryStream};
use pin_project_lite::pin_project;

use crate::{and::And, dest::Dest, Source};

pub trait Unit {
    type Future<'a>: Future<Output = ()>
    where
        Self: 'a;
    fn run<'a>(self) -> Self::Future<'a>;
}

pub trait UnitExt: Unit {
    fn and<T>(self, next: T) -> And<Self, T>
    where
        Self: Sized,
        T: Unit,
    {
        And::new(self, next)
    }
}

impl<T> UnitExt for T where T: Unit {}

pub struct SourceUnit<S, T> {
    source: S,
    dest: T,
}

impl<S, T> SourceUnit<S, T> {
    pub fn new(source: S, dest: T) -> SourceUnit<S, T> {
        SourceUnit { source, dest }
    }
}

impl<S, T> Unit for SourceUnit<S, T>
where
    T: Dest<S::Item> + 'static,
    S: Source + 'static,
{
    type Future<'a> = SourceUnitFure<'a, S, T>;

    fn run<'a>(self) -> Self::Future<'a> {
        SourceUnitFure {
            stream: self.source.call(),
            dest: self.dest,
            future: None,
        }
    }
}

pin_project! {
    pub struct SourceUnitFure<'a, S: 'a, T:'a> where T: Dest<S::Item>, S: Source {
        #[pin]
        stream: S::Stream<'a>,
        dest: T,
        #[pin]
        future: Option<T::Future<'a>>
    }
}

impl<'a, S, T> Future for SourceUnitFure<'a, S, T>
where
    T: Dest<S::Item> + 'a,
    S: Source,
{
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.poll(cx));
                this.future.set(None);
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)) {
                if let Ok(ok) = item {
                    this.future.set(Some(this.dest.call(ok)));
                }
            } else {
                break ();
            }
        })
    }
}
