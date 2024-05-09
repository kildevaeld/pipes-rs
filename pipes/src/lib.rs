#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod and;
mod cloned;
mod cond;
mod context;
mod dest;
mod error;
#[cfg(feature = "fs")]
pub mod fs;
mod func;
#[cfg(feature = "std")]
mod package;
mod pipeline;
mod source;
mod unit;
mod work;
mod work_many;

#[cfg(feature = "http")]
pub mod http;

pub use self::{
    cond::*, context::Context, dest::*, error::Error, pipeline::Pipeline, source::*, unit::*,
    work::*, work_many::*,
};

#[cfg(feature = "std")]
pub use self::package::*;

pub mod prelude {
    pub use super::{SourceExt, UnitExt, WorkExt};
}
