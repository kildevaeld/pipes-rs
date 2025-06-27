use std::convert::Infallible;

use arbejd::{Work, prelude::*, when, work_fn};

#[tokio::main]
async fn main() {
    let test = when(
        |v: &u32| *v == 41,
        work_fn(|ctx: i32, req: u32| async move {
            Result::<_, Infallible>::Ok(format!("Hello, {req}: {ctx}"))
        }),
    );

    // let test = work_fn(|ctx: i32, req: u32| async move {
    //     Result::<_, Infallible>::Ok(format!("Hello, {req}: {ctx}"))
    // });

    let handler = test
        .pipe(work_fn(
            |ctx, req| async move { Ok(format!("Hello {req}")) },
        ))
        .boxed();

    let out = tokio::spawn(async move { handler.call(&100, 42).await }).await;

    println!("{:?}", out);
}
