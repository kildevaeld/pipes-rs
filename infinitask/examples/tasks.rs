use async_timer::oneshot::{Oneshot, Timer};
use infinitask::{task_fn, Delegate, InifiniTask, Task, TaskCtx, TaskId};

#[derive(Debug, Clone)]
struct Ctx;

#[derive(Debug, Clone)]
struct Logger;

#[async_trait::async_trait]
impl Delegate<Ctx> for Logger {
    async fn task_registered(&self, _ctx: &Ctx, id: TaskId, _task: &dyn infinitask::Task<Ctx>) {
        println!("task registered {:?}", id)
    }

    async fn task_started(&self, _ctx: &Ctx, id: TaskId) {
        println!("task started {:?}", id)
    }

    async fn task_finished(&self, _ctx: &Ctx, id: TaskId, error: Option<infinitask::TaskError>) {
        println!("task finished {:?}: {:?}", id, error)
    }
}

fn main() {
    let tasks = InifiniTask::<Ctx, _>::new(Logger);

    futures::executor::block_on(tasks.run(
        Ctx,
        task_fn(|ctx: TaskCtx<Ctx>| async move {
            println!("Task!");

            ctx.register(task_fn(|_ctx: TaskCtx<Ctx>| async move {
                let work = Timer::new(std::time::Duration::from_millis(100));

                work.await;

                println!("Other task");
                Ok(())
            }))
            .await;

            ctx.register(task_fn(|_ctx: TaskCtx<Ctx>| async move {
                println!("Other task 2");
                Ok(())
            }))
            .await;

            Ok(())
        }),
    ))
}
