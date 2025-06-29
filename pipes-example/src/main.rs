use bycat_error::Error;
use bytes::Bytes;

use bycat::{
    prelude::{WorkExt, *},
    work_fn,
};
use bycat_fs::FsDest;
use bycat_http::{HttpWork, get};
use bycat_img::{Format, Operation};
use bycat_package::{Package, prelude::WorkExt as _};
use bycat_source::{Unit, pipe, prelude::*};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let stream = pipe::<(),_>(vec![get("https://loppen.dk/sites/default/files/styles/wide-image/public/Ki%21%20foto%20fra%20Frontcover%20copy%20%281%29.jpg?itok=FoSCWMbQ")].pipe(HttpWork::default().into_package()));

    stream
        .pipe(bycat_img::load())
        .cloned(
            bycat_img::imageop(vec![
                Operation::Resize {
                    width: 200,
                    height: 1200,
                },
                Operation::Blur { sigma: 2. },
            ])
            .pipe(bycat_img::save(Format::Webp {
                quality: 75.,
                lossless: false,
            })),
            bycat_img::save(Format::Jpg(80)),
        )
        .then(work_fn(|_ctx, ret: Result<Package<_>, Error>| async move {
            match ret {
                Ok(ret) => Ok(ret),
                Err(err) => {
                    println!("Got error: {err}");
                    Err(err)
                }
            }
        }))
        .pipe(FsDest::new("test"))
        .unit()
        .run(&())
        .await;
}
