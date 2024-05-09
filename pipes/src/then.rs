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

impl<T1, T2, R> Work<R> for Then<T1, T2>
where
    T1: Work<R>,
    T2: Work<Result<T1::Output, Error>> + Clone,
{
    type Output = T2::Output;

    type Future = ThenWorkFuture<T1, T2, R>;

    fn call(&self, ctx: crate::Context, package: R) -> Self::Future {
        ThenWorkFuture::Left {
            future: self.left.call(ctx.clone(), package),
            next: Some(self.right.clone()),
            ctx: Some(ctx),
        }
    }
}

impl<T1, T2> Source for Then<T1, T2>
where
    T1: Source + 'static,
    T2: Work<Result<T1::Item, Error>> + 'static + Clone,
{
    type Item = T2::Output;

    type Stream<'a> = ThenStream<T1::Stream<'a>, T2, T1::Item>;

    fn call<'a>(self) -> Self::Stream<'a> {
        ThenStream {
            stream: self.left.call(),
            work: self.right.clone(),
            future: None,
            ctx: Context {},
        }
    }
}

pin_project! {
    #[project = ThenWorkProject]
    pub enum ThenWorkFuture<T1, T2, R>
    where
    T1: Work<R>,
    T2: Work<Result<T1::Output, Error>>,
    {
        Left {
            #[pin]
            future: T1::Future,
            next: Option<T2>,
            ctx: Option<Context>,
        },
        Right {
            #[pin]
            future: T2::Future,
        },
        Done
    }
}

impl<T1, T2, R> Future for ThenWorkFuture<T1, T2, R>
where
    T1: Work<R>,
    T2: Work<Result<T1::Output, Error>>,
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
    pub struct ThenStream<T, W, R> where W: Work<Result<R, Error>> {
        #[pin]
        stream: T,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: Context
    }
}

impl<T, W, R> Stream for ThenStream<T, W, R>
where
    W: Work<Result<R, Error>>,
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
