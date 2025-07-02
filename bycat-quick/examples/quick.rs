use bycat_container::modules::{BuildContext, Builder, InitContext, Module};
use bycat_error::Error;
use bycat_quick::{JsBuilderContext, JsInitContext, Quick};

pub struct TestModule;

impl<'js> Module<'js, JsBuilderContext<'js>> for TestModule {
    fn build<'a>(
        self,
        ctx: &'a mut JsBuilderContext<'js>,
    ) -> impl Future<Output = Result<(), bycat_error::Error>> + 'a
    where
        Self: 'a,
    {
        async move {
            //

            ctx.ctx
                .globals()
                .set("Rapper", "nahah")
                .map_err(Error::new)?;

            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut builder = Quick::default();

    builder.add(|ctx: &mut JsInitContext<'_>| {
        //
        ctx.add_module(TestModule);

        bycat_error::Result::Ok(())
    });

    let vm = builder.build().await?;

    klaver::async_with!(vm => |ctx| {
        let rapper: String = ctx.globals().get("Rapper")?;
        println!("Rapper {}", rapper);
        Ok(())
    })
    .await
    .map_err(Error::new)?;

    Ok(())
}
