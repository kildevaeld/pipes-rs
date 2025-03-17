use async_channel as channel;
use async_trait::async_trait;
use futures::{
    pin_mut,
    stream::{FuturesUnordered, StreamExt},
};
use std::{
    any::Any,
    future::Future,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> TaskId {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TaskId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{inner}")]
pub struct TaskError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl TaskError {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(err: E) -> TaskError {
        TaskError {
            inner: Box::new(err),
        }
    }
}

struct TaskRequest<C> {
    id: TaskId,
    chan: channel::Sender<TaskRequest<C>>,
    task: Box<dyn Task<C>>,
}

#[derive(Debug, Clone)]
pub struct TaskCtx<C> {
    context: C,
    chan: channel::Sender<TaskRequest<C>>,
}

impl<C> TaskCtx<C> {
    pub fn data(&self) -> &C {
        &self.context
    }

    pub fn data_mut(&mut self) -> &mut C {
        &mut self.context
    }

    pub async fn register<T>(&self, task: T)
    where
        T: Task<C> + 'static + Send + Sync,
    {
        self.chan
            .send(TaskRequest {
                id: TaskId::new(),
                chan: self.chan.clone(),
                task: Box::new(task),
            })
            .await
            .ok();
    }

    pub fn register_blocking<T>(&self, task: T)
    where
        T: Task<C> + 'static + Send + Sync,
    {
        self.chan
            .send_blocking(TaskRequest {
                id: TaskId::new(),
                chan: self.chan.clone(),
                task: Box::new(task),
            })
            .ok();
    }
}

#[async_trait]
pub trait Task<C>: Send + Sync {
    async fn run(self: Box<Self>, context: TaskCtx<C>) -> Result<(), TaskError>;

    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
impl<C: Sync + Send> Task<C> for Box<dyn Task<C> + Send> {
    async fn run(self: Box<Self>, context: TaskCtx<C>) -> Result<(), TaskError> {
        (*self).run(context).await
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
}

#[async_trait]
pub trait TaskList<C> {
    async fn register(self, ctx: TaskCtx<C>);
}

#[async_trait]
impl<T, C> TaskList<C> for (T,)
where
    T: Task<C> + 'static + Send + Sync,
    C: Sync + Send + 'static,
{
    async fn register(self, ctx: TaskCtx<C>) {
        ctx.register(self.0).await
    }
}

#[async_trait]
impl<T, C> TaskList<C> for Vec<T>
where
    T: Task<C> + 'static + Send + Sync,
    C: Sync + Send + 'static,
{
    async fn register(self, ctx: TaskCtx<C>) {
        for next in self {
            ctx.register(next).await;
        }
    }
}

#[async_trait]
pub trait Delegate<C> {
    async fn task_registered(&self, ctx: &C, id: TaskId, task: &dyn Task<C>);
    async fn task_started(&self, ctx: &C, task: TaskId);
    async fn task_finished(&self, ctx: &C, task: TaskId, error: Option<TaskError>);
}

pub trait Spawner {
    type Future<T>: Future;
    fn spawn<T: Future>(&self) -> Self::Future<T>;
}

#[async_trait]
impl<C> Delegate<C> for () {
    async fn task_registered(&self, _ctx: &C, _id: TaskId, _task: &dyn Task<C>) {}
    async fn task_started(&self, _ctx: &C, _task: TaskId) {}
    async fn task_finished(&self, _ctx: &C, _task: TaskId, _error: Option<TaskError>) {}
}

pub struct InifiniTask<C, D> {
    ctx: PhantomData<C>,
    delegate: D,
}

impl<C> Default for InifiniTask<C, ()> {
    fn default() -> Self {
        InifiniTask {
            ctx: PhantomData,
            delegate: (),
        }
    }
}

impl<C, D> InifiniTask<C, D> {
    pub fn new(delegate: D) -> InifiniTask<C, D> {
        InifiniTask {
            ctx: PhantomData,
            delegate,
        }
    }
}

impl<C, D> InifiniTask<C, D>
where
    D: Delegate<C> + 'static,
    C: Send + Sync + 'static + Clone,
{
    pub async fn run<T>(&self, ctx: C, task: T)
    where
        T: Task<C> + Send + 'static + Sync,
    {
        self.run_many(ctx, (task,)).await
    }

    pub async fn run_many<T>(&self, ctx: C, tasks: T)
    where
        T: TaskList<C>,
    {
        let (sx, rx) = channel::unbounded::<TaskRequest<C>>();
        pin_mut!(rx);

        tasks
            .register(TaskCtx {
                context: ctx.clone(),
                chan: sx,
            })
            .await;

        let mut queue = FuturesUnordered::new();

        let ctx = &ctx;
        let delegate = &self.delegate;

        loop {
            futures::select! {
                next = rx.next() => {
                    let Some(next) = next else {
                        continue
                    };

                    delegate.task_registered(&ctx, next.id, &*next.task).await;

                    queue.push(async move {
                        delegate.task_started(&ctx, next.id).await;
                        let ret = next.task.run(TaskCtx {
                            context: ctx.clone(),
                            chan: next.chan
                        }).await;

                        (next.id, ret)
                    });
                }

                next = queue.next() => {
                    let Some((task, ret)) = next else {
                        //break;
                        continue;
                    };

                    delegate.task_finished(&ctx, task, ret.err()).await;


                }
                complete  => break

            };
        }
    }
}

pub struct TaskFn<C, F, U> {
    ph: PhantomData<(C, U)>,
    func: F,
}

unsafe impl<C, F: Send, U> Send for TaskFn<C, F, U> {}

unsafe impl<C, F: Sync, U> Sync for TaskFn<C, F, U> {}

#[async_trait]
impl<C, F, U> Task<C> for TaskFn<C, F, U>
where
    for<'a> F: FnOnce(TaskCtx<C>) -> U + Send + Sync,
    for<'a> U: Future<Output = Result<(), TaskError>> + Send + 'a,
    C: Send + Sync + 'static,
    F: 'static,
{
    async fn run(self: Box<Self>, ctx: TaskCtx<C>) -> Result<(), TaskError> {
        (self.func)(ctx).await
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn task_fn<F, U, C>(func: F) -> TaskFn<C, F, U>
where
    F: FnOnce(TaskCtx<C>) -> U,
    U: Future<Output = Result<(), TaskError>>,
{
    TaskFn {
        ph: PhantomData,
        func,
    }
}
