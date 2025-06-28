#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod and;
// mod cloned;
// mod error;
mod pipeline;
mod source;
mod then;
mod unit;
// mod wrap;

pub use bycat::{work_fn, NoopWork, Work};

pub use self::{
    // cloned::*,
    // error::Result,
    // error::*,
    pipeline::Pipeline,
    source::*,
    unit::*,
};

pub mod prelude {
    pub use super::{SourceExt, UnitExt};
    pub use bycat::prelude::*;
}

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork<T::Error>, C>
where
    T: Source<C>,
{
    Pipeline::new(source)
}
