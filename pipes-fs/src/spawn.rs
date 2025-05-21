use futures::{Stream, StreamExt, pin_mut};
use pin_project_lite::pin_project;
use pipes::{Error, Source, Work};
use tokio::sync::mpsc;
pub struct Spawn<S, W> {
    source: S,
    work: W,
}

impl<S, W, C> Source<C> for Spawn<S, W>
where
    S: Source<C> + Send + 'static,
    S::Item: Send,
    for<'a> S::Stream<'a>: Send,
    W: Work<C, S::Item> + Send + 'static,
    W::Output: Send,
    for<'a> W::Future<'a>: Send,
    C: Clone + Send + 'static,
{
    type Item = W::Output;

    type Stream<'a>
        = SpawnStream<C, S, W>
    where
        Self: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        let (sx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let stream = self.source.call(ctx.clone());
            pin_mut!(stream);
            while let Some(item) = stream.next().await {
                let ret = match item {
                    Ok(ret) => self.work.call(ctx.clone(), ret).await,
                    Err(err) => Err(err),
                };

                if sx.send(ret).await.is_err() {
                    break;
                }
            }
        });

        SpawnStream { rx }
    }
}

pin_project! {
    pub struct SpawnStream<C, T, W> where W: Work<C, T::Item>, T: Source<C> {
        rx: mpsc::Receiver<Result<W::Output, Error>>
    }
}

impl<C, T, W> Stream for SpawnStream<C, T, W>
where
    T: Source<C>,
    W: Work<C, T::Item>,
{
    type Item = Result<W::Output, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}
