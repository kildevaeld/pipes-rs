#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod and;
mod cloned;
mod error;
mod matcher;
mod pipeline;
mod source;
// mod split;
mod then;
mod unit;
// mod when;
// mod work;
// mod work_many;
mod wrap;

use arbejd::NoopWork;

pub use self::{
    cloned::*, error::Result, error::*, matcher::*, pipeline::Pipeline, source::*, unit::*,
};

pub mod prelude {
    pub use super::{SourceExt, UnitExt};
    pub use arbejd::prelude::*;
}

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork, C> {
    Pipeline::new(source)
}
