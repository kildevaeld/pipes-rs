use core::marker::PhantomData;

use bycat::Work;
use bycat_error::Error;
use bytes::Bytes;

use crate::{Content, IntoPackage, Package, into_package::IntoPackageWork};

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

pub struct ContentIntoBytes<T> {
    work: T,
}

impl<T, C, I> Work<C, Package<I>> for ContentIntoBytes<T>
where
    T: Work<C, I>,
    I: Content,
{
    type Output = Package<Bytes>;

    type Error = Error;

    type Future<'a>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Package<I>) -> Self::Future<'a> {
        todo!()
    }
}
