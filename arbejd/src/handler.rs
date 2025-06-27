use core::marker::PhantomData;
use heather::{HBoxFuture, HSend, HSendSync, Hrc};

pub trait Handler<I, C>: HSendSync {
    type Output;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Output, Self::Error>> + HSend
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: I) -> Self::Future<'a>;
}

// Boxed
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
pub fn box_handler<'a, C, B, T>(handler: T) -> BoxHandler<'a, B, C>
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
pub fn box_handler<C, B, T>(handler: T) -> BoxHandler<'static, B, C>
where
    T: Handler<B, C> + 'static,
    B: HSend + 'static,
    C: HSendSync + 'static,
{
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
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> HBoxFuture<'a, Result<Response<B>, Error>> {
        Box::pin(async move { self.0.call(context, req).await.map(|m| m.into_response()) })
    }
}

#[cfg(feature = "send")]
impl<B, C, T> DynHandler<B, C> for HandlerBox<B, C, T>
where
    T: Handler<B, C> + 'static,
    C: HSendSync + 'static,
    B: HSend,
{
    fn call<'a>(
        &'a self,
        context: &'a C,
        req: Request<B>,
    ) -> HBoxFuture<'a, Result<Response<B>, Error>> {
        Box::pin(async move { self.0.call(context, req).await.map(|m| m.into_response()) })
    }
}

pub struct BoxHandler<'a, B, C> {
    inner: Hrc<dyn DynHandler<B, C> + 'a>,
}

unsafe impl<'a, B, C> Send for BoxHandler<'a, B, C> where Hrc<dyn DynHandler<B, C> + 'a>: Send {}

unsafe impl<'a, B, C> Sync for BoxHandler<'a, B, C> where Hrc<dyn DynHandler<B, C> + 'a>: Sync {}

impl<'c, B, C> Handler<B, C> for BoxHandler<'c, B, C> {
    type Response = Response<B>;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Response, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: Request<B>) -> Self::Future<'a> {
        self.inner.call(context, req)
    }
}

impl<'a, B, C> Clone for BoxHandler<'a, B, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
