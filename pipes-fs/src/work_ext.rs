use std::marker::PhantomData;

use pipes::Work;

use crate::{IntoPackage, IntoPackageWork};

pub trait FsWorkExt<C, T>: Work<C, T> {
    fn into_package(self) -> IntoPackageWork<Self, C>
    where
        Self: Sized,
        Self::Output: IntoPackage,
    {
        IntoPackageWork {
            worker: self,
            ctx: PhantomData,
        }
    }
}

impl<C, T, W> FsWorkExt<C, T> for W where W: Work<C, T> {}
