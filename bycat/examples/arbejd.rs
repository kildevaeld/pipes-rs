use std::convert::Infallible;

use bycat::{Work, box_work, prelude::*, when, work_fn};

#[tokio::main]
async fn main() {
    let test = when(
        |v: &u32| *v == 42,
        work_fn(|ctx: i32, req: u32| async move {
            Result::<_, Infallible>::Ok(format!("Hello, {req}: {ctx}"))
        }),
    )
    .map_err(|_err| "Fejlede");

    // let test = work_fn(|ctx: i32, req: u32| async move {
    //     Result::<_, Infallible>::Ok(format!("Hello, {req}: {ctx}"))
    // });

    let handler = test.pipe(work_fn(
        |ctx, req| async move { Ok(format!("Hello {req}")) },
    ));

    let handler = box_work(handler);

    let out = handler.call(&100, 42).await; //tokio::spawn(async move { handler.call(&100, 42).await }).await;

    println!("{:?}", out);
}
