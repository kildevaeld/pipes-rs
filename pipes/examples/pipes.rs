use futures::{StreamExt, TryStreamExt};
use pipes::{pipe, work_fn, Error, Source, SourceExt, Unit};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pipe = pipe(vec![Result::<_, Error>::Ok("Hello")])
        .pipe(work_fn(|ctx, pkg| async move {
            println!("Work {pkg}");
            pipes::Result::Ok("Other")
        }))
        .pipe(work_fn(|ctx, pkg| async move {
            println!("Work 2: {pkg}");
            pipes::Result::Ok("next other")
        }));

    pipe.start(())
        .try_for_each_concurrent(10, |rx| async move {
            //
            println!("Output {}", rx);
            Ok(())
        })
        .await
        .unwrap();

    // pipe.run(()).await;
}
