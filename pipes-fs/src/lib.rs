mod dest;
// mod into_package;
// mod package;
mod body;
mod resolver;
#[cfg(feature = "serde")]
mod serialize;
mod source;
mod work;

#[cfg(feature = "serde")]
pub use self::serialize::Serde;

pub use self::{body::Body, dest::*, source::FsSource, work::*};

pub use mime::{self, Mime};
