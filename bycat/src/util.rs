use core::task::{Poll, ready};

use either::Either;
use futures_core::Stream;
use pin_project_lite::pin_project;

pub trait IntoEither {
    type Left;
    type Right;

    fn into_either(self) -> Either<Self::Left, Self::Right>;
}

impl<L, R> IntoEither for Either<L, R> {
    type Left = L;
    type Right = R;
    fn into_either(self) -> Either<L, R> {
        self
    }
}

pub trait IntoResult {
    type Output;
    type Error;

    fn into_result(self) -> Result<Self::Output, Self::Error>;
}

impl<T, E> IntoResult for Result<T, E> {
    type Error = E;
    type Output = T;
    fn into_result(self) -> Result<Self::Output, Self::Error> {
        self
    }
}

impl<L, R> IntoResult for Either<L, R>
where
    L: IntoResult,
    R: IntoResult<Output = L::Output, Error = L::Error>,
{
    type Error = L::Error;
    type Output = L::Output;
    fn into_result(self) -> Result<Self::Output, Self::Error> {
        match self {
            Either::Left(left) => left.into_result(),
            Either::Right(right) => right.into_result(),
        }
    }
}

pub struct ResultIter<T>(pub T);

impl<T> Iterator for ResultIter<T>
where
    T: Iterator,
    T::Item: IntoResult,
{
    type Item = Result<<T::Item as IntoResult>::Output, <T::Item as IntoResult>::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|item| item.into_result())
    }
}

impl<T> ResultStream<T> {
    pub fn new(stream: T) -> ResultStream<T> {
        ResultStream { stream }
    }
}

pin_project! {
    pub struct ResultStream<T> {
        #[pin]
        stream: T
    }
}

impl<T> Stream for ResultStream<T>
where
    T: Stream,
    T::Item: IntoResult,
{
    type Item = Result<<T::Item as IntoResult>::Output, <T::Item as IntoResult>::Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        match ready!(self.project().stream.poll_next(cx)) {
            Some(item) => Poll::Ready(Some(item.into_result())),
            None => Poll::Ready(None),
        }
    }
}
