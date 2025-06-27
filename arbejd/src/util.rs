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
