use either::Either;

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
