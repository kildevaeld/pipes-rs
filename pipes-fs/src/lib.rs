mod dest;
mod into_package;
mod package;
mod resolver;
#[cfg(feature = "serde")]
mod serialize;
mod source;
mod work;
mod work_ext;

#[cfg(feature = "serde")]
pub use self::serialize::Serde;

pub use self::{
    dest::*,
    into_package::IntoPackageWork,
    package::{Body, IntoPackage, Meta, Package},
    source::FsSource,
    work::*,
};

pub use mime::{self, Mime};

pub mod prelude {
    pub use super::work_ext::*;
}
