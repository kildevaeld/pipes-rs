use core::task::Poll;
use futures_core::{Future, ready};
use pin_project_lite::pin_project;

use crate::Work;

#[derive(Debug, Clone, Copy)]
pub struct And<T1, T2> {
    pub left: T1,
    pub right: T2,
}

impl<T1, T2> And<T1, T2> {
    pub fn new(left: T1, right: T2) -> And<T1, T2> {
        And { left, right }
    }
}

impl<T1, T2, C, R> Work<C, R> for And<T1, T2>
where
    T1: Work<C, R>,
    T2: Work<C, T1::Output, Error = T1::Error>,
{
    type Output = T2::Output;
    type Error = T2::Error;
    type Future<'a>
        = AndWorkFuture<'a, T1, T2, C, R>
    where
        Self: 'a,
        C: 'a;
    fn call<'a>(&'a self, ctx: &'a C, package: R) -> Self::Future<'a> {
        AndWorkFuture::Left {
            future: self.left.call(ctx, package),
            next: &self.right,
            ctx: Some(ctx),
        }
    }
}

pin_project! {
    #[project = AndWorkProject]
    pub enum AndWorkFuture<'a, T1: 'a, T2: 'a, C, R>
    where
    T1: Work<C, R>,
    T2: Work<C,T1::Output>,
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

impl<'a, T1, T2, C, R> Future for AndWorkFuture<'a, T1, T2, C, R>
where
    T1: Work<C, R>,
    T2: Work<C, T1::Output, Error = T1::Error>,
{
    type Output = Result<T2::Output, T2::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match this {
                AndWorkProject::Left { future, next, ctx } => {
                    let ret = match ready!(future.poll(cx)) {
                        Ok(ret) => ret,
                        Err(err) => {
                            self.set(AndWorkFuture::Done);
                            return Poll::Ready(Err(err));
                        }
                    };

                    let ctx = ctx.take().unwrap();
                    let future = next.call(ctx, ret);
                    self.set(AndWorkFuture::Right { future });
                }
                AndWorkProject::Right { future } => {
                    let ret = ready!(future.poll(cx));
                    self.set(AndWorkFuture::Done);
                    return Poll::Ready(ret);
                }
                AndWorkProject::Done => {
                    panic!("poll after done")
                }
            }
        }
    }
}
