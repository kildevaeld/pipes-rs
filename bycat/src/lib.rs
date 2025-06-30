#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
pub mod map_err;
mod matcher;
mod middleware;
mod middleware_fn;
pub mod pipe;
pub mod split;
pub mod stream;
pub mod then;
#[cfg(feature = "tower")]
mod tower;
mod util;
pub mod when;
mod work;
mod work_ext;
mod work_fn;
pub use self::{
    matcher::Matcher, middleware::*, middleware_fn::*, util::*, when::when, work::*, work_fn::*,
};

#[cfg(feature = "alloc")]
pub use self::boxed::{BoxWork, box_work};

#[cfg(feature = "tower")]
pub use self::tower::{Tower, TowerFuture};

pub mod prelude {
    pub use super::work_ext::*;
}
