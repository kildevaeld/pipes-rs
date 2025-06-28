#![no_std]

extern crate alloc;

mod content;
mod ext;
mod into_package;
mod matcher;
mod package;

pub use self::{
    content::*,
    into_package::{IntoPackageWork, IntoPackageWorkFuture},
    matcher::*,
    package::{IntoPackage, Meta, Package},
};
pub mod prelude {
    pub use super::ext::*;
}

pub use bytes::{self, Bytes};
pub use mime::{self, Mime};

pub use async_trait::async_trait;
