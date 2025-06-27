use super::work::Work;
use alloc::boxed::Box;
use core::marker::PhantomData;
use heather::{HBoxFuture, HSend, HSendSync, Hrc};

pub trait DynWork<C, B>: HSendSync {
    type Output;
    type Error;

    fn call<'a>(
        &'a self,
        context: &'a C,
        req: B,
    ) -> HBoxFuture<'a, Result<Self::Output, Self::Error>>;
}

#[cfg(not(feature = "send"))]
pub fn box_work<'a, C, B, T>(handler: T) -> BoxWork<'a, C, B, T::Output, T::Error>
where
    T: Work<C, B> + 'a,
    B: HSend + 'a,
    C: HSendSync + 'a,
{
    BoxWork {
        inner: Hrc::from(WorkBox(handler, PhantomData, PhantomData)),
    }
}

#[cfg(feature = "send")]
pub fn box_work<C, B, T>(handler: T) -> BoxWork<'static, C, B, T::Output, T::Error>
where
    T: Work<C, B> + HSendSync + 'static,
    B: HSend + 'static,
    C: HSendSync + 'static,
    for<'a> T::Future<'a>: Send,
{
    BoxWork {
        inner: Hrc::from(WorkBox(handler, PhantomData, PhantomData)),
    }
}

pub struct WorkBox<C, B, T>(T, PhantomData<C>, PhantomData<B>);

unsafe impl<B, C, T: Send> Send for WorkBox<B, C, T> {}

unsafe impl<B, C, T: Sync> Sync for WorkBox<B, C, T> {}

#[cfg(not(feature = "send"))]
impl<C, B, T> DynWork<C, B> for WorkBox<C, B, T>
where
    T: Work<C, B>,
    C: HSendSync,
    B: HSend,
{
    type Error = T::Error;
    type Output = T::Output;
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: B,
    ) -> HBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move { self.0.call(context, req).await })
    }
}

#[cfg(feature = "send")]
impl<B, C, T> DynWork<C, B> for WorkBox<C, B, T>
where
    T: Work<C, B> + HSendSync + 'static,
    C: HSendSync + 'static,
    B: HSend,
    for<'a> T::Future<'a>: Send,
{
    type Error = T::Error;
    type Output = T::Output;
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: B,
    ) -> HBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move { self.0.call(context, req).await })
    }
}

pub struct BoxWork<'a, C, B, O, E> {
    inner: Hrc<dyn DynWork<C, B, Error = E, Output = O> + 'a>,
}

unsafe impl<'a, B, C, O, E> Send for BoxWork<'a, C, B, O, E> where
    Hrc<dyn DynWork<C, B, Output = O, Error = E> + 'a>: Send
{
}

unsafe impl<'a, C, B, O, E> Sync for BoxWork<'a, C, B, O, E> where
    Hrc<dyn DynWork<C, B, Output = O, Error = E> + 'a>: Sync
{
}

impl<'c, C, B, O, E> Work<C, B> for BoxWork<'c, C, B, O, E> {
    type Output = O;
    type Error = E;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Output, Self::Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: B) -> Self::Future<'a> {
        self.inner.call(context, req)
    }
}

impl<'a, B, C, O, E> Clone for BoxWork<'a, B, C, O, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
