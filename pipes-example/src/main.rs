use bycat_error::Error;
use bytes::Bytes;

use bycat::{NoopWork, Work, prelude::*, work_fn};
use bycat_fs::{Body, FsDest};
use bycat_http::{HttpWork, get};
use bycat_img::{Format, ImageWork, Operation};
use bycat_package::{Content, Package, prelude::WorkExt};
use bycat_source::{Unit, pipe, prelude::*};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let stream = pipe::<(),_>(vec![get("https://loppen.dk/sites/default/files/styles/wide-image/public/Ki%21%20foto%20fra%20Frontcover%20copy%20%281%29.jpg?itok=FoSCWMbQ")].pipe(HttpWork::default().into_package()));

    stream
        // .pipe(ImageWork::default().wrap(|ctx, pkg, task| async move {
        //     println!("wrap task: {}", pkg.name());
        //     task.call(ctx, pkg).await
        // }))
        // .cloned(
        //     bycat_img::imageop(vec![
        //         Operation::Resize {
        //             width: 200,
        //             height: 1200,
        //         },
        //         Operation::Blur { sigma: 2. },
        //     ])
        //     .pipe(bycat_img::save(Format::Webp {
        //         quality: 75.,
        //         lossless: false,
        //     })),
        //     bycat_img::save(Format::Jpg(80)),
        // )
        .then(work_fn(|_ctx, ret: Result<Package<_>, Error>| async move {
            match ret {
                Ok(ret) => Ok(ret),
                Err(err) => {
                    println!("Got error");
                    Err(err)
                }
            }
        }))
        .pipe(work_fn(|_ctx, ret: Package<_>| async move {
            bycat_error::Result::Ok(
                ret.map(
                    |mut body: bycat_package::StreamContent<bycat_http::BodyStream>| async move {
                        Body::Bytes(body.bytes().await.unwrap())
                    },
                )
                .await,
            )
        }))
        .pipe(FsDest::new("test"))
        .unit()
        .run(&())
        .await;
}
