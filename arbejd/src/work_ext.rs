use crate::{Work, map_err::MapErr, pipe::And, split::Split, then::Then, util::IntoEither};

pub trait WorkExt<C, I>: Work<C, I> {
    fn pipe<T>(self, next: T) -> And<Self, T>
    where
        Self: Sized,
        T: Work<C, Self::Output>,
    {
        And::new(self, next)
    }

    fn then<T>(self, next: T) -> Then<Self, T>
    where
        Self: Sized,
        T: Work<C, Result<Self::Output, Self::Error>>,
    {
        Then::new(self, next)
    }

    fn split<L, R>(self, left: L, right: R) -> Split<Self, L, R>
    where
        Self: Sized,
        Self::Output: IntoEither,
        L: Work<C, <Self::Output as IntoEither>::Left, Error = Self::Error> + Clone,
        R: Work<C, <Self::Output as IntoEither>::Right, Output = L::Output, Error = Self::Error>
            + Clone,
        C: Clone,
    {
        Split::new(self, left, right)
    }

    fn map_err<T, E>(self, map: T) -> MapErr<Self, T, E>
    where
        Self: Sized,
        T: Fn(Self::Error) -> E,
    {
        MapErr::new(self, map)
    }
}

impl<C, I, T> WorkExt<C, I> for T where T: Work<C, I> {}
