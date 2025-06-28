use core::marker::PhantomData;

use bycat::Work;

use crate::{IntoPackage, into_package::IntoPackageWork};

pub trait WorkExt<C, T>: Work<C, T> {
    fn into_package<B>(self) -> IntoPackageWork<Self, C, B>
    where
        Self: Sized,
        Self::Output: IntoPackage<B>,
    {
        IntoPackageWork {
            worker: self,
            ctx: PhantomData,
        }
    }
}

impl<C, T, W> WorkExt<C, T> for W where W: Work<C, T> {}
