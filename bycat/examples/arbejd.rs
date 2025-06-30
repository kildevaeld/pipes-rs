use std::{convert::Infallible, fmt::Display};

use bycat::{BoxWork, Middleware, Work, box_work, middleware, prelude::*, when, work_fn};

struct M;

impl<C: Clone, B, H: Clone> Middleware<C, B, H> for M
where
    H: Work<C, B>,
    H::Output: Display,
    H: 'static,
    B: 'static,
    C: 'static,
{
    type Work = BoxWork<'static, C, B, String, H::Error>;

    fn wrap(&self, handler: H) -> Self::Work {
        box_work(work_fn(move |ctx: C, req| {
            let handler = handler.clone();
            async move {
                let handler = handler.call(&ctx, req).await?;

                Result::<_, H::Error>::Ok(format!("Middleare {}", handler))
            }
        }))
    }
}

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

    let handler = test
        .pipe(work_fn(
            |ctx, req| async move { Ok(format!("Hello {req}")) },
        ))
        .wrap(M);

    let handler = box_work(handler);

    let out = handler.call(&100, 42).await; //tokio::spawn(async move { handler.call(&100, 42).await }).await;

    println!("{:?}", out);
}
