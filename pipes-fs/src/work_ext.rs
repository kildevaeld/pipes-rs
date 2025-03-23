use std::marker::PhantomData;

use pipes::Work;

use crate::{IntoPackage, IntoPackageWork};

pub trait FsWorkExt<C, T>: Work<C, T> {
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

impl<C, T, W> FsWorkExt<C, T> for W where W: Work<C, T> {}
