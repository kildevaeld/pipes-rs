#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod and;
mod cloned;
mod cond;
mod error;
mod matcher;
mod pipeline;
mod source;
mod split;
mod then;
mod unit;
mod work;
// mod work_many;
mod wrap;

pub use self::{
    cloned::*, cond::*, error::Result, error::*, matcher::*, pipeline::Pipeline, source::*,
    then::*, unit::*, work::*,
};

pub mod prelude {
    pub use super::{SourceExt, UnitExt, WorkExt};
}

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork, C> {
    Pipeline::new(source)
}
