use crate::{IntoEither, map_err::MapErr, split::Split, then::Then, util::IntoResult};
use core::{
    marker::PhantomData,
    mem::transmute,
    task::{Poll, ready},
};
use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::{Work, pipe::And};

pub struct StreamBuilder<C, I, T> {
    work: T,
    data: PhantomData<fn() -> (C, I)>,
}

impl<C, I, T: Clone> Clone for StreamBuilder<C, I, T> {
    fn clone(&self) -> Self {
        Self {
            work: self.work.clone(),
            data: PhantomData,
        }
    }
}

impl<C, I, T: Copy> Copy for StreamBuilder<C, I, T> {}

impl<C, I, T> StreamBuilder<C, I, T> {
    pub fn new(task: T) -> StreamBuilder<C, I, T> {
        StreamBuilder {
            work: task,
            data: PhantomData,
        }
    }
}

impl<C, I, T> StreamBuilder<C, I, T>
where
    T: Work<C, I>,
{
    pub fn pipe<W>(self, work: W) -> StreamBuilder<C, I, And<T, W>>
    where
        W: Work<C, T::Output, Error = T::Error>,
    {
        StreamBuilder {
            work: And::new(self.work, work),
            data: PhantomData,
        }
    }

    pub fn then<W>(self, work: W) -> StreamBuilder<C, I, Then<T, W>>
    where
        W: Work<C, Result<T::Output, T::Error>, Error = T::Error>,
    {
        StreamBuilder {
            work: Then::new(self.work, work),
            data: PhantomData,
        }
    }

    pub fn split<L, R>(self, left: L, right: R) -> StreamBuilder<C, I, Split<T, L, R>>
    where
        T::Output: IntoEither,
        L: Work<C, <T::Output as IntoEither>::Left, Error = T::Error> + Clone,
        R: Work<C, <T::Output as IntoEither>::Right, Output = L::Output, Error = T::Error> + Clone,
        C: Clone,
    {
        StreamBuilder {
            work: Split::new(self.work, left, right),
            data: self.data,
        }
    }

    pub fn map_err<F, E>(self, map: F) -> StreamBuilder<C, I, MapErr<T, F, E>>
    where
        Self: Sized,
        F: Fn(T::Error) -> E,
    {
        StreamBuilder {
            work: MapErr::new(self.work, map),
            data: self.data,
        }
    }

    pub fn build<S>(self, ctx: C, stream: S) -> WorkStream<T, S, C>
    where
        S: Stream,
        S::Item: IntoResult,
        <S::Item as IntoResult>::Error:
            Into<<T as Work<C, <S::Item as IntoResult>::Output>>::Error>,
        T: Work<C, <S::Item as IntoResult>::Output>,
        S: Stream,
    {
        WorkStream {
            stream,
            work: self.work,
            ctx,
            future: State::Init,
        }
    }
}

pin_project! {
  #[project = StateProj]
  enum State<T> {
    Future {
      #[pin]
      future: T },
    Init,
  }
}

pin_project! {
  pub struct WorkStream<T: 'static, S, C: 'static>
where
    S: Stream,
    S::Item: IntoResult,
    T: Work<C, <S::Item as IntoResult>::Output>
{
    #[pin]
    stream: S,
    work: T,
    ctx: C,
    #[pin]
    future: State<T::Future<'static>>,
}
}

impl<T, S, C> Stream for WorkStream<T, S, C>
where
    S: Stream,
    S::Item: IntoResult,
    <S::Item as IntoResult>::Error: Into<T::Error>,
    T: Work<C, <S::Item as IntoResult>::Output> + 'static,
    C: 'static,
    S: Stream,
{
    type Item = Result<T::Output, T::Error>;

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        loop {
            match self.as_mut().project().future.as_mut().project() {
                StateProj::Future { future } => {
                    let ret = ready!(future.poll(cx));
                    self.as_mut().project().future.set(State::Init);
                    return Poll::Ready(Some(ret));
                }
                StateProj::Init => {}
            }

            let mut this = self.as_mut().project();

            match ready!(this.stream.poll_next(cx)) {
                Some(ret) => {
                    let ret = match ret.into_result() {
                        Ok(ret) => ret,
                        Err(err) => return Poll::Ready(Some(Err(err.into()))),
                    };

                    let future = this.work.call(&this.ctx, ret);

                    this.future.set(State::Future {
                        future: unsafe { transmute::<T::Future<'_>, T::Future<'static>>(future) },
                    });
                }
                None => return Poll::Ready(None),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
