use std::{marker::PhantomData, str::FromStr};

use pipes::{BoxError, Work};

pub struct ParseStr<T>(PhantomData<T>);

impl<C, V, T> Work<C, V> for ParseStr<T>
where
    T: FromStr,
    T::Err: Into<BoxError>,
    V: AsRef<str>,
{
    type Output = T;

    type Future<'a>
        = core::future::Ready<Result<Self::Output, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, _ctx: C, package: V) -> Self::Future<'a> {
        let ret = T::from_str(package.as_ref()).map_err(pipes::Error::new);
        core::future::ready(ret)
    }
}

pub fn parse_str<T>() -> ParseStr<T>
where
    T: FromStr,
    T::Err: Into<BoxError>,
{
    ParseStr(PhantomData)
}
