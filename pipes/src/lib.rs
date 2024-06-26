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
#[cfg(feature = "std")]
mod package;
mod pipeline;
mod source;
mod split;
mod then;
mod unit;
mod utils;
mod work;
mod work_many;
mod wrap;

#[cfg(feature = "http")]
pub mod http;

pub use self::{
    cond::*, context::Context, dest::*, error::Error, pipeline::Pipeline, source::*, then::*,
    unit::*, work::*, work_many::*,
};

#[cfg(feature = "std")]
pub use self::package::*;

pub mod prelude {
    pub use super::{SourceExt, UnitExt, WorkExt};
}
