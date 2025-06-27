#[cfg(feature = "alloc")]
use crate::{BoxWork, box_work};
use crate::{Work, pipe::And, then::Then};
#[cfg(all(feature = "alloc", feature = "send"))]
use heather::{HSend, HSendSync};
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

    #[cfg(all(feature = "alloc", not(feature = "send")))]
    fn boxed<'a>(self) -> BoxWork<'a, C, I, Self::Output, Self::Error>
    where
        Self: Sized + 'a,
        I: 'a,
        C: 'a,
    {
        box_work(self)
    }

    #[cfg(all(feature = "alloc", feature = "send"))]
    fn boxed(self) -> BoxWork<'static, C, I, Self::Output, Self::Error>
    where
        Self: HSendSync + Sized + 'static,
        I: HSend + 'static,
        C: HSendSync + 'static,
        for<'a> Self::Future<'a>: HSend,
    {
        box_work(self)
    }
}

impl<C, I, T> WorkExt<C, I> for T where T: Work<C, I> {}
