mod ext;
mod into_package;
mod matcher;
mod package;

pub use self::{
    into_package::{IntoPackageWork, IntoPackageWorkFuture},
    matcher::*,
    package::{IntoPackage, Meta, Package},
};
pub mod prelude {
    pub use super::ext::*;
}
