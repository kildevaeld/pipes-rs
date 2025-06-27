use core::task::Poll;
use futures_core::{Future, ready};
use pin_project_lite::pin_project;

use crate::Work;

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
    T2: Work<C, Result<T1::Output, T1::Error>> + Clone,
    C: Clone,
{
    type Output = T2::Output;
    type Error = T2::Error;

    type Future<'a>
        = ThenWorkFuture<'a, T1, T2, C, R>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, ctx: &'a C, package: R) -> Self::Future<'a> {
        ThenWorkFuture::Left {
            future: self.left.call(ctx, package),
            next: &self.right,
            ctx: Some(ctx),
        }
    }
}

pin_project! {
    #[project = ThenWorkProject]
    pub enum ThenWorkFuture<'a, T1: 'a , T2: 'a, C, R>
    where
    T1: Work<C, R>,
    T2: Work<C,Result<T1::Output, T1::Error>>,
    {
        Left {
            #[pin]
            future: T1::Future<'a>,
            next: &'a T2,
            ctx: Option<&'a C>,
        },
        Right {
            #[pin]
            future: T2::Future<'a>,
        },
        Done
    }
}

impl<'a, T1, T2, C, R> Future for ThenWorkFuture<'a, T1, T2, C, R>
where
    T1: Work<C, R>,
    T2: Work<C, Result<T1::Output, T1::Error>>,
{
    type Output = Result<T2::Output, T2::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match this {
                ThenWorkProject::Left { future, next, ctx } => {
                    let ret = ready!(future.poll(cx));

                    // let next = next.take().expect("next");
                    let ctx = ctx.take().expect("context");
                    let future = next.call(ctx, ret);
                    self.set(ThenWorkFuture::Right { future });
                }
                ThenWorkProject::Right { future, .. } => {
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
