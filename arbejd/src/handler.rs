use heather::{HSend, HSendSync};

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
