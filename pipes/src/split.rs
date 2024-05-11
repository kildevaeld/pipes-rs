use core::{marker::PhantomData, pin::Pin, task::Poll};
use std::sync::Mutex;

use alloc::{collections::VecDeque, sync::Arc};
use either::Either;
use futures::{ready, Stream, TryFuture, TryStream};
use pin_project_lite::pin_project;

use crate::{Error, Source, Work};

pub trait Splited {
    type Left;
    type Right;

    fn into_either(self) -> Either<Self::Left, Self::Right>;
}

impl<L, R> Splited for Either<L, R> {
    type Left = L;
    type Right = R;
    fn into_either(self) -> Either<L, R> {
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Split<S, W> {
    source: S,
    work: W,
}

impl<S, W> Split<S, W> {
    pub fn new(source: S, work: W) -> Split<S, W> {
        Split { source, work }
    }
}

impl<S, W, C> Source<C> for Split<S, W>
where
    S: Source<C>,
    W: Work<C, S::Item> + Clone + 'static,
    W::Output: Splited,
    C: Clone,
{
    type Item = Either<<W::Output as Splited>::Left, <W::Output as Splited>::Right>;

    type Stream<'a> = SplitStream<'a, S, W, C> where S: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        SplitStream {
            stream: self.source.call(ctx.clone()),
            work: self.work.clone(),
            future: None,
            ctx,
        }
    }
}

pin_project! {
    pub struct SplitStream<'a, S: 'a, W, C>
    where
        S: Source<C>,
        W: Work<C, S::Item>,
        W::Output: Splited,
     {
        #[pin]
        stream: S::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future>,
        ctx: C
     }
}

impl<'a, S: 'a, W, C> Stream for SplitStream<'a, S, W, C>
where
    S: Source<C>,
    W: Work<C, S::Item>,
    W::Output: Splited,
    C: Clone,
{
    type Item = Result<Either<<W::Output as Splited>::Left, <W::Output as Splited>::Right>, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                break Some(item.map(|m| m.into_either()));
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.future
                    .set(Some(this.work.call(this.ctx.clone(), item)));
            } else {
                break None;
            }
        })
    }
}

pin_project! {
    pub struct Split2<'a, S: 'a, C>
    where
        S: Source<C>,
        S::Item: Splited,
    {
        #[pin]
        source: S::Stream<'a>,
        left: VecDeque<<S::Item as Splited>::Left>,
        right: VecDeque<<S::Item as Splited>::Right>,
        ctx: PhantomData<C>,
    }
}

impl<'a, S: 'a, C> Split2<'a, S, C>
where
    S: Source<C>,
    S::Item: Splited,
{
    pub fn next_left(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Result<<S::Item as Splited>::Left, Error>>> {
        let mut this = self.project();

        if let Some(next) = this.left.pop_front() {
            return Poll::Ready(Some(Ok(next)));
        }

        loop {
            match ready!(this.source.as_mut().try_poll_next(cx)) {
                Some(Err(err)) => return Poll::Ready(Some(Err(err))),
                Some(Ok(ret)) => match ret.into_either() {
                    Either::Left(left) => return Poll::Ready(Some(Ok(left))),
                    Either::Right(right) => {
                        this.right.push_back(right);
                    }
                },
                None => return Poll::Ready(None),
            }
        }
    }

    pub fn next_right(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Result<<S::Item as Splited>::Right, Error>>> {
        let mut this = self.project();

        if let Some(next) = this.right.pop_front() {
            return Poll::Ready(Some(Ok(next)));
        }

        loop {
            match ready!(this.source.as_mut().try_poll_next(cx)) {
                Some(Err(err)) => return Poll::Ready(Some(Err(err))),
                Some(Ok(ret)) => match ret.into_either() {
                    Either::Right(right) => return Poll::Ready(Some(Ok(right))),
                    Either::Left(left) => {
                        this.left.push_back(left);
                    }
                },
                None => return Poll::Ready(None),
            }
        }
    }
}

pin_project! {
    pub struct LeftStream<'a, S, C>
    where
        S: Source<C>,
        S::Item: Splited,
    {
        inner:  Arc<Mutex<Split2<'a, S, C>>>
    }
}

impl<'a, S, C> Stream for LeftStream<'a, S, C>
where
    S: Source<C>,
    S::Item: Splited,
{
    type Item = Result<<S::Item as Splited>::Left, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut guard = this.inner.lock().expect("lock");
        let projected = unsafe { Pin::new_unchecked(&mut *guard) };
        projected.next_left(cx)
    }
}

pin_project! {
    pub struct RightStream<'a, S, C>
    where
        S: Source<C>,
        S::Item: Splited,
    {
        inner: Arc<Mutex<Split2<'a, S, C>>>
    }
}

impl<'a, S, C> Stream for RightStream<'a, S, C>
where
    S: Source<C>,
    S::Item: Splited,
{
    type Item = Result<<S::Item as Splited>::Right, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut guard = this.inner.lock().expect("lock");
        let projected = unsafe { Pin::new_unchecked(&mut *guard) };
        projected.next_right(cx)
    }
}

pub struct Left<'a, S, C>
where
    S: Source<C>,
    S::Item: Splited,
{
    split: Arc<Mutex<Split2<'a, S, C>>>,
}

pub fn split<S, W, C>(source: S, work: W)
where
    S: Source<C>,
    W: Work<C, S::Item>,
    W::Output: Splited,
{
}
