use crate::{error::Error, source::Source, work::Work, Unit};
use core::task::Poll;
use futures::{ready, Future};
use pin_project_lite::pin_project;

#[derive(Debug, Clone, Copy)]
pub struct And<T1, T2> {
    left: T1,
    right: T2,
}

impl<T1, T2> And<T1, T2> {
    pub fn new(left: T1, right: T2) -> And<T1, T2> {
        And { left, right }
    }
}

impl<T1, T2, C, R> Work<C, R> for And<T1, T2>
where
    T1: Work<C, R> + 'static,
    T2: Work<C, T1::Output> + 'static,
    C: Clone,
{
    type Output = T2::Output;
    type Future<'a> = AndWorkFuture<'a, T1, T2, C, R>;
    fn call<'a>(&'a self, ctx: C, package: R) -> Self::Future<'a> {
        AndWorkFuture::Left {
            future: self.left.call(ctx.clone(), package),
            next: &self.right,
            ctx: Some(ctx),
        }
    }
}

impl<T1, T2, C> Source<C> for And<T1, T2>
where
    T1: Source<C>,
    T2: Source<C, Item = T1::Item>,
    C: Clone,
{
    type Item = T1::Item;
    type Stream<'a> = futures::stream::Select<T1::Stream<'a>, T2::Stream<'a>> where T1: 'a, T2: 'a;
    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        futures::stream::select(self.left.call(ctx.clone()), self.right.call(ctx))
    }
}

impl<T1, T2, C> Unit<C> for And<T1, T2>
where
    T1: Unit<C>,
    T2: Unit<C>,
    C: Clone,
{
    type Future<'a> = AndUnitFuture<T1::Future<'a>, T2::Future<'a>> where T1: 'a, T2: 'a;

    fn run<'a>(self, ctx: C) -> Self::Future<'a> {
        AndUnitFuture {
            future: futures::future::join(self.left.run(ctx.clone()), self.right.run(ctx)),
        }
    }
}

pin_project! {
    #[project = AndWorkProject]
    pub enum AndWorkFuture<'a, T1: 'static, T2: 'static, C, R>
    where
    T1: Work<C, R>,
    T2: Work<C,T1::Output>,
    {
        Left {
            #[pin]
            future: T1::Future<'a>,
            next: &'a T2,
            ctx: Option<C>,
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
    T2: Work<C, T1::Output>,
{
    type Output = Result<T2::Output, Error>;

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

pin_project! {
    pub struct AndUnitFuture<T1, T2> where T1: Future<Output = ()>, T2: Future<Output = ()> {
        #[pin]
        future: futures::future::Join<T1, T2>
    }
}

impl<T1, T2> Future for AndUnitFuture<T1, T2>
where
    T1: Future<Output = ()>,
    T2: Future<Output = ()>,
{
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.future.poll(cx));
        Poll::Ready(())
    }
}
