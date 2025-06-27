#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
mod matcher;
pub mod pipe;
pub mod then;
mod util;
pub mod when;
mod work;
mod work_ext;
mod work_fn;
pub use self::{matcher::Matcher, when::when, work::*, work_fn::*};

#[cfg(feature = "alloc")]
pub use self::boxed::{BoxWork, box_work};

pub mod prelude {
    pub use super::work_ext::*;
}
