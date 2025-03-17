use std::marker::PhantomData;

use async_trait::async_trait;
use futures::{StreamExt, stream::BoxStream};
use infinitask::InifiniTask;
use pipes::Source;

pub struct InifiniSource<T, R>(T, PhantomData<R>);

impl<T, R> InifiniSource<T, R> {
    pub fn new(task: T) -> InifiniSource<T, R> {
        InifiniSource(task, PhantomData)
    }
}

impl<T: 'static, R: 'static, C> Source<C> for InifiniSource<T, R>
where
    T: Task<C, R> + Send + Sync,
    C: Clone + Send + Sync + 'static,
    R: Send,
{
    type Item = R;

    type Stream<'a>
        = BoxStream<'a, Result<Self::Item, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        async_stream::stream! {

            let (sx, rx) = async_channel::unbounded();

            let tasks = InifiniTask::new(());

            tasks
                .run(
                    ctx,
                    infinitask::task_fn(move |ctx| async move {
                        let ret = Box::new(self.0)
                            .run(TaskCtx {
                                inner: ctx,
                                sx: sx.clone(),
                            })
                            .await;

                        sx.send(ret).await.map_err(|err| panic!(""))
                    }),
                )
                .await;


            while let Ok(next) = rx.recv().await {
                yield next
            }


        }
        .boxed()
    }
}

#[derive(Debug, Clone)]
pub struct TaskCtx<C, R> {
    inner: infinitask::TaskCtx<C>,
    sx: async_channel::Sender<Result<R, pipes::Error>>,
}

impl<C: Clone + Send + Sync + 'static, R> TaskCtx<C, R> {
    pub fn data(&self) -> &C {
        self.inner.data()
    }

    pub fn data_mut(&mut self) -> &mut C {
        self.inner.data_mut()
    }

    pub async fn register<T>(&self, task: T)
    where
        T: Task<C, R> + 'static + Send + Sync,
        R: Send + 'static,
    {
        let sx = self.sx.clone();
        self.inner
            .register(infinitask::task_fn(move |ctx| async move {
                //
                let ret = Box::new(task)
                    .run(TaskCtx {
                        inner: ctx,
                        sx: sx.clone(),
                    })
                    .await;

                sx.send(ret).await.map_err(|err| panic!(""))
            }))
            .await;
    }
}

#[async_trait]
pub trait Task<C, R> {
    async fn run(self: Box<Self>, context: TaskCtx<C, R>) -> Result<R, pipes::Error>;
}

pub struct TaskFn<C, F, U, R> {
    ph: PhantomData<(C, U, R)>,
    func: F,
}

#[async_trait]
impl<C, F, U, R> Task<C, R> for TaskFn<C, F, U, R>
where
    for<'a> F: FnOnce(TaskCtx<C, R>) -> U + Send + Sync,
    for<'a> U: Future<Output = Result<R, pipes::Error>> + Send + 'a,
    C: Send + Sync + 'static,
    R: Send,
    F: 'static,
{
    async fn run(self: Box<Self>, ctx: TaskCtx<C, R>) -> Result<R, pipes::Error> {
        (self.func)(ctx).await
    }
}

pub fn task_fn<F, U, C, R>(func: F) -> TaskFn<C, F, U, R>
where
    F: FnOnce(TaskCtx<C, R>) -> U,
    U: Future<Output = Result<R, pipes::Error>>,
{
    TaskFn {
        ph: PhantomData,
        func,
    }
}
