use bytes::Bytes;

use pipes::{NoopWork, Pipeline, Unit, Work, WorkExt as _, prelude::*, work_fn};
use pipes_fs::{Body, FsDest};
use pipes_http::{HttpWork, get};
use pipes_img::{Format, ImageWork, Operation};
use pipes_package::{Package, prelude::WorkExt};

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork, C> {
    Pipeline::new(source)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let stream = pipe::<(),_>(vec![get("https://loppen.dk/sites/default/files/styles/wide-image/public/Ki%21%20foto%20fra%20Frontcover%20copy%20%281%29.jpg?itok=FoSCWMbQ")].pipe(HttpWork::default().into_package()));

    stream
        .pipe(ImageWork::default().wrap(|ctx, pkg, task| async move {
            println!("wrap task: {}", pkg.name());
            task.call(ctx, pkg).await
        }))
        .cloned(
            pipes_img::imageop(vec![
                Operation::Resize {
                    width: 200,
                    height: 1200,
                },
                Operation::Blur { sigma: 2. },
            ])
            .pipe(pipes_img::save(Format::Webp {
                quality: 75.,
                lossless: false,
            })),
            pipes_img::save(Format::Jpg(80)),
        )
        .then(work_fn(
            |_ctx, ret: Result<Package<Bytes>, pipes::Error>| async move {
                match ret {
                    Ok(ret) => Ok(ret),
                    Err(err) => {
                        println!("Got error");
                        Err(err)
                    }
                }
            },
        ))
        .pipe(work_fn(|_ctx, ret: Package<Bytes>| async move {
            pipes::Result::Ok(ret.map(|body| async move { Body::Bytes(body) }).await)
        }))
        .pipe(FsDest::new("test"))
        .unit()
        .run(())
        .await;
}
