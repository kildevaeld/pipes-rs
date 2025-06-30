use bycat::{BoxWork, Work, box_work, work_fn};
use bycat_container::modules::{Backend, BoxModule, BuildContext, Builder, InitContext, ModuleBox};
use bycat_error::{Error, Result};
use std::collections::BTreeMap;

pub struct Test;

impl Backend for Test {
    type BuildContext<'ctx> = TestBuildContext<'ctx>;
    type InitContext<'ctx> = TestInitContext<'ctx>;
}

#[derive(Default)]
pub struct TestBuildContext<'ctx> {
    handlers: BTreeMap<String, BoxWork<'ctx, TestRunContext, i32, i32, Error>>,
}

impl<'ctx> BuildContext<'ctx> for TestBuildContext<'ctx> {
    type Context = TestRunContext;
}

impl<'ctx> TestBuildContext<'ctx> {
    pub fn add_handler<T>(&mut self, name: impl Into<String>, handler: T)
    where
        for<'c> T: Work<TestRunContext, i32, Output = i32, Error = Error> + 'c,
    {
        self.handlers.insert(name.into(), box_work(handler));
    }
}

#[derive(Clone)]
pub struct TestRunContext {}

#[derive(Default)]
pub struct TestInitContext<'ctx> {
    modules: Vec<BoxModule<'ctx, TestBuildContext<'ctx>>>,
}

impl<'ctx> InitContext<'ctx> for TestInitContext<'ctx> {
    type Backend = Test;

    fn add_module<T>(&mut self, module: T)
    where
        T: bycat_container::modules::Module<
                'ctx,
                bycat_container::modules::BuildContextType<'ctx, Self::Backend>,
            > + 'ctx,
    {
        self.modules.push(ModuleBox::new(module));
    }
}

fn main() {
    let mut builder = Builder::<Test>::default();

    builder.add(|ctx: &mut TestInitContext<'_>| {
        //
        ctx.add_module(|ctx: &mut TestBuildContext| {
            println!("Add module");
            ctx.add_handler(
                "test",
                work_fn(|ctx: TestRunContext, input: i32| async move {
                    //
                    Result::Ok(42)
                }),
            );
            Ok(())
        });
        Ok(())
    });

    let mut ctx = TestInitContext::default();

    futures::executor::block_on(async move {
        builder.build(&mut ctx).await?;

        let mut build_ctx = TestBuildContext::default();

        for module in ctx.modules {
            module.build(&mut build_ctx).await?;
        }

        let ret = build_ctx
            .handlers
            .get("test")
            .unwrap()
            .call(&TestRunContext {}, 100)
            .await?;

        println!("Ret {}", ret);

        Result::Ok(())
    })
    .unwrap();
}
