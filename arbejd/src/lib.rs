#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
mod handler;
mod handler_fn;

pub use self::{handler::*, handler_fn::*};

#[cfg(feature = "alloc")]
pub use self::boxed::{BoxHandler, box_handler};
