use core::task::Poll;

use futures::{ready, Future, Stream};
use pin_project_lite::pin_project;

use crate::{Context, Error, Source, Work};

#[derive(Debug, Clone, Copy)]
pub struct Then<T1, T2> {
    left: T1,
    right: T2,
}

impl<T1, T2> Then<T1, T2> {
    pub fn new(left: T1, right: T2) -> Then<T1, T2> {
        Then { left, right }
    }
}

impl<T1, T2, C, R> Work<C, R> for Then<T1, T2>
where
    T1: Work<C, R>,
    T2: Work<C, Result<T1::Output, Error>> + Clone,
    C: Clone,
{
    type Output = T2::Output;

    type Future = ThenWorkFuture<T1, T2, C, R>;

    fn call(&self, ctx: C, package: R) -> Self::Future {
        ThenWorkFuture::Left {
            future: self.left.call(ctx.clone(), package),
            next: Some(self.right.clone()),
            ctx: Some(ctx),
        }
    }
}

impl<T1, T2, C> Source<C> for Then<T1, T2>
where
    T1: Source<C> + 'static,
    T2: Work<C, Result<T1::Item, Error>> + 'static + Clone,
    C: Clone,
{
    type Item = T2::Output;

    type Stream<'a> = ThenStream<T1::Stream<'a>, T2, C, T1::Item>;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        ThenStream {
            stream: self.left.call(ctx.clone()),
            work: self.right.clone(),
            future: None,
            ctx: ctx,
        }
    }
}

pin_project! {
    #[project = ThenWorkProject]
    pub enum ThenWorkFuture<T1, T2, C, R>
    where
    T1: Work<C, R>,
    T2: Work<C,Result<T1::Output, Error>>,
    {
        Left {
            #[pin]
            future: T1::Future,
            next: Option<T2>,
            ctx: Option<C>,
        },
        Right {
            #[pin]
            future: T2::Future,
        },
        Done
    }
}

impl<T1, T2, C, R> Future for ThenWorkFuture<T1, T2, C, R>
where
    T1: Work<C, R>,
    T2: Work<C, Result<T1::Output, Error>>,
{
    type Output = Result<T2::Output, Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match this {
                ThenWorkProject::Left { future, next, ctx } => {
                    let ret = ready!(future.poll(cx));

                    let next = next.take().expect("next");
                    let ctx = ctx.take().expect("context");
                    self.set(ThenWorkFuture::Right {
                        future: next.call(ctx, ret),
                    });
                }
                ThenWorkProject::Right { future } => {
                    let ret = ready!(future.poll(cx));
                    self.set(ThenWorkFuture::Done);
                    return Poll::Ready(ret);
                }
                ThenWorkProject::Done => {
                    panic!("poll after done")
                }
            }
        }
    }
}

pin_project! {
    pub struct ThenStream<T, W , C, R> where W: Work<C,Result<R, Error>> {
        #[pin]
        stream: T,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: C
    }
}

impl<T, W, C, R> Stream for ThenStream<T, W, C, R>
where
    W: Work<C, Result<R, Error>>,
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
                let item = ready!(fut.poll(cx));
                this.future.set(None);
                break Some(item);
            } else if let Some(item) = ready!(this.stream.as_mut().poll_next(cx)) {
                this.future
                    .set(Some(this.work.call(this.ctx.clone(), item)));
            } else {
                break None;
            }
        })
    }
}
