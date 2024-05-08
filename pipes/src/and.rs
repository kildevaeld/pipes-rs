use crate::{context::Context, error::Error, source::Source, work::Work, Unit};
use core::task::Poll;
use futures::{ready, Future};
use pin_project_lite::pin_project;

pub struct And<T1, T2> {
    left: T1,
    right: T2,
}

impl<T1, T2> And<T1, T2> {
    pub fn new(left: T1, right: T2) -> And<T1, T2> {
        And { left, right }
    }
}

impl<T1, T2, R> Work<R> for And<T1, T2>
where
    T1: Work<R>,
    T2: Work<T1::Output> + Clone,
{
    type Output = T2::Output;
    type Future = AndWorkFuture<T1, T2, R>;
    fn call(&self, ctx: Context, package: R) -> Self::Future {
        AndWorkFuture::Left {
            future: self.left.call(ctx.clone(), package),
            next: Some(self.right.clone()),
            ctx: Some(ctx),
        }
    }
}

impl<T1, T2> Source for And<T1, T2>
where
    T1: Source,
    T2: Source<Item = T1::Item>,
{
    type Item = T1::Item;
    type Stream = futures::stream::Select<T1::Stream, T2::Stream>;
    fn call(self) -> Self::Stream {
        futures::stream::select(self.left.call(), self.right.call())
    }
}

impl<T1, T2> Unit for And<T1, T2>
where
    T1: Unit,
    T2: Unit,
{
    type Future<'a> = AndUnitFuture<T1::Future<'a>, T2::Future<'a>> where T1: 'a, T2: 'a;

    fn run<'a>(self) -> Self::Future<'a> {
        AndUnitFuture {
            future: futures::future::join(self.left.run(), self.right.run()),
        }
    }
}

pin_project! {
    #[project = AndWorkProject]
    pub enum AndWorkFuture<T1, T2, R>
    where
    T1: Work<R>,
    T2: Work<T1::Output>,
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

impl<T1, T2, R> Future for AndWorkFuture<T1, T2, R>
where
    T1: Work<R>,
    T2: Work<T1::Output>,
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

                    let next = next.take().unwrap();
                    let ctx = ctx.take().unwrap();
                    self.set(AndWorkFuture::Right {
                        future: next.call(ctx, ret),
                    });
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
