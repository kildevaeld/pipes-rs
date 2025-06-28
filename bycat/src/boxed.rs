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

// #[cfg(not(feature = "send"))]
// pub fn box_work<'a, C, B, T>(handler: T) -> BoxWork<'a, C, B, T::Output, T::Error>
// where
//     T: Work<C, B> + 'a,
//     B: HSend + 'a,
//     C: HSendSync + 'a,
// {
//     BoxWork {
//         inner: Hrc::from(WorkBox(handler, PhantomData, PhantomData)),
//     }
// }

// #[cfg(feature = "send")]
pub fn box_work<'c, C, B, T>(handler: T) -> BoxWork<'c, C, B, T::Output, T::Error>
where
    T: Work<C, B> + HSendSync + 'c,
    B: HSend + 'c,
    C: HSendSync + 'c,
    for<'a> T::Future<'a>: HSend,
{
    BoxWork {
        inner: Hrc::from(WorkBox(handler, PhantomData, PhantomData)),
    }
}

pub struct WorkBox<C, B, T>(T, PhantomData<C>, PhantomData<B>);

unsafe impl<B, C, T: Send> Send for WorkBox<B, C, T> {}

unsafe impl<B, C, T: Sync> Sync for WorkBox<B, C, T> {}

// #[cfg(not(feature = "send"))]
// impl<C, B, T> DynWork<C, B> for WorkBox<C, B, T>
// where
//     T: Work<C, B>,
//     C: HSendSync,
//     B: HSend,
// {
//     type Error = T::Error;
//     type Output = T::Output;
//     fn call<'a>(
//         &'a self,
//         context: &'a C,
//         req: B,
//     ) -> HBoxFuture<'a, Result<Self::Output, Self::Error>> {
//         Box::pin(async move { self.0.call(context, req).await })
//     }
// }

// #[cfg(feature = "send")]
impl<B, C, T> DynWork<C, B> for WorkBox<C, B, T>
where
    T: Work<C, B> + HSendSync,
    C: HSendSync,
    B: HSend,
    for<'a> T::Future<'a>: HSend,
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

// Middleware

// pub fn box_middleware<'a, B, C, M, H>(middleware: M) -> BoxMiddleware<'a, B, C, H>
// where
//     M: Middleware<B, C, H> + 'a,
//     M::Work: 'a,
//     B: HSend + 'a,
//     C: HSendSync + 'a,
// {
//     BoxMiddleware {
//         inner: Hrc::from(MiddlewareBox(middleware, PhantomData)),
//     }
// }

// struct MiddlewareBox<'a, C, B, T>(T, PhantomData<&'a (B, C)>);

// impl<'a, B, C, T: Clone> Clone for MiddlewareBox<'a, B, C, T> {
//     fn clone(&self) -> Self {
//         MiddlewareBox(self.0.clone(), PhantomData)
//     }
// }

// unsafe impl<'a, B, C, T: Send> Send for MiddlewareBox<'a, B, C, T> {}

// unsafe impl<'a, B, C, T: Sync> Sync for MiddlewareBox<'a, B, C, T> {}

// impl<'a, B, C, T, H> Middleware<C, B, H> for MiddlewareBox<'a, C, B, T>
// where
//     T: Middleware<C, B, H>,
//     T::Work: 'a,
//     B: HSend + 'a,
//     C: HSendSync + 'a,
// {
//     type Work = BoxWork<'a, C, B, <T::Work as Work<C, B>>::Output, <T::Work as Work<C, B>>::Error>;

//     fn wrap(&self, handle: H) -> Self::Work {
//         let handle = self.0.wrap(handle);
//         box_work(handle)
//     }
// }

// pub struct BoxMiddleware<'a, C, B, H> {
//     inner: Hrc<dyn Middleware<C, B, H, Work = BoxWork<'a, C, B>> + 'a>,
// }

// unsafe impl<'a, B, C, H> Send for BoxMiddleware<'a, C, B, H> where
//     Hrc<dyn Middleware<C, B, H, Handle = BoxWork<'a, C, B>>>: Send
// {
// }

// unsafe impl<'a, B, C, H> Sync for BoxMiddleware<'a, C, B, H> where
//     Hrc<dyn Middleware<C, B, H, Handle = BoxWork<'a, C, B>>>: Sync
// {
// }

// impl<'a, B, C, H> Middleware<C, B, H> for BoxMiddleware<'a, C, B, H> {
//     type Work = BoxWork<'a, C, B>;

//     fn wrap(&self, handle: H) -> Self::Work {
//         self.inner.wrap(handle)
//     }
// }

// impl<'a, B, C, H> Clone for BoxMiddleware<'a, B, C, H> {
//     fn clone(&self) -> Self {
//         BoxMiddleware {
//             inner: self.inner.clone(),
//         }
//     }
// }
