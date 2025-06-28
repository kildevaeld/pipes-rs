use std::convert::Infallible;

use futures::TryStreamExt;
use pipes::{iter, pipe, prelude::*, work_fn, Source};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pipe = pipe(iter(vec![Result::<_, Infallible>::Ok("Hello")]))
        .pipe(work_fn(|ctx, pkg| async move {
            println!("Work {pkg}");
            Result::<_, Infallible>::Ok("Other")
        }))
        .pipe(work_fn(|ctx, pkg| async move {
            println!("Work 2: {pkg}");
            Result::<_, Infallible>::Ok("next other")
        }));

    pipe.create_stream(&())
        .try_for_each(|rx| async move {
            //
            println!("Output {}", rx);
            Ok(())
        })
        .await
        .unwrap();

    // pipe.run(()).await;
}
