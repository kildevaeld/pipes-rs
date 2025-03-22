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
// #[cfg(feature = "fs")]
// pub mod fs;
// #[cfg(feature = "std")]
// mod package;
mod pipeline;
mod source;
mod split;
mod then;
mod unit;
mod utils;
mod work;
mod work_many;
mod wrap;

// #[cfg(feature = "http")]
// pub mod http;

pub use self::{
    cloned::*, cond::*, context::Context, dest::*, error::Error, pipeline::Pipeline, source::*,
    then::*, unit::*, work::*, work_many::*,
};

// #[cfg(feature = "std")]
// pub use self::package::*;

// #[cfg(feature = "std")]
// pub use relative_path::{RelativePath, RelativePathBuf};

pub mod prelude {
    pub use super::{SourceExt, UnitExt, WorkExt};
}

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork, C> {
    Pipeline::new(source)
}
