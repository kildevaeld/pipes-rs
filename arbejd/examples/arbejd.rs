use std::convert::Infallible;

use arbejd::{Handler, box_handler, work_fn};

#[tokio::main]
async fn main() {
    let test = work_fn(|ctx: &(), req: u32| async move {
        Result::<_, Infallible>::Ok(format!("Hello, {req}"))
    });

    let handler = box_handler(test);

    let out = tokio::spawn(async move { handler.call(&(), 42).await }).await;

    println!("{:?}", out);
}
