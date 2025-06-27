use super::handler::Handler;
use alloc::boxed::Box;
use core::marker::PhantomData;
use heather::{HBoxFuture, HSend, HSendSync, Hrc};

pub trait DynHandler<B, C>: HSendSync {
    type Output;
    type Error;

    fn call<'a>(
        &'a self,
        context: &'a C,
        req: B,
    ) -> HBoxFuture<'a, Result<Self::Output, Self::Error>>;
}

#[cfg(not(feature = "send"))]
pub fn box_handler<'a, C, B, T>(handler: T) -> BoxHandler<'a, B, C, T::Output, T::Error>
where
    T: Handler<B, C> + 'a,
    B: HSend + 'static,
    C: HSendSync + 'a,
{
    BoxHandler {
        inner: Hrc::from(HandlerBox(handler, PhantomData, PhantomData)),
    }
}

#[cfg(feature = "send")]
pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<'static, B, C, T::Output, T::Error>
where
    T: Handler<B, C> + 'static,
    B: HSend + 'static,
    C: HSendSync + 'static,
{
    use crate::Handler;

    BoxHandler {
        inner: Hrc::from(HandlerBox(handler, PhantomData, PhantomData)),
    }
}

pub struct HandlerBox<B, C, T>(T, PhantomData<C>, PhantomData<B>);

unsafe impl<B, C, T: Send> Send for HandlerBox<B, C, T> {}

unsafe impl<B, C, T: Sync> Sync for HandlerBox<B, C, T> {}

#[cfg(not(feature = "send"))]
impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C>,
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
impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C> + 'static,
    C: HSendSync + 'static,
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

pub struct BoxHandler<'a, B, C, O, E> {
    inner: Hrc<dyn DynHandler<B, C, Error = E, Output = O> + 'a>,
}

unsafe impl<'a, B, C, O, E> Send for BoxHandler<'a, B, C, O, E> where
    Hrc<dyn DynHandler<B, C, Output = O, Error = E> + 'a>: Send
{
}

unsafe impl<'a, B, C, O, E> Sync for BoxHandler<'a, B, C, O, E> where
    Hrc<dyn DynHandler<B, C, Output = O, Error = E> + 'a>: Sync
{
}

impl<'c, B, C, O, E> Handler<B, C> for BoxHandler<'c, B, C, O, E> {
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

impl<'a, B, C, O, E> Clone for BoxHandler<'a, B, C, O, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
