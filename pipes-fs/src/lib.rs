mod dest;
mod into_package;
mod package;
mod resolver;
mod source;
mod work;
mod work_ext;

pub use self::{
    dest::FsDest,
    into_package::IntoPackageWork,
    package::{Body, IntoPackage, Meta, Package},
    source::FsSource,
    work::*,
};

pub use mime::{self, Mime};

pub mod prelude {
    pub use super::work_ext::*;
}
