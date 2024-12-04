use boa_engine::{property::Attribute, Context, JsArgs, JsResult, JsValue, NativeFunction, Source};
use boa_runtime::Console;
use std::{future::Future, rc::Rc};

// use pleiades_worker::runtime::job_queue;

#[tokio::main]
async fn main() -> JsResult<()> {
    // Instantiate the execution context
    let mut context = init();

    // code
    let code = r#"

        new Promise((resolve) => {
            sleep(5);
        }).then((value) => {
            console.log(value);
        });

        new Promise((resolve) => {
            sleep(4);
        }).then((value) => {
            resolve("Hello, World3!");
        });

        // (async () => {
        //     await sleep();
        // })();

        // let counter = 0;
        // for (let i = 0; i < 100000; i++) {
        //     counter += 1;
        // }
        // console.log(counter);

        //counter
    "#;

    let result = context.eval(Source::from_bytes(code))?;
    // context.run_jobs();
    context.run_jobs_async().await;
    println!("{}", result.display());

    Ok(())
}

fn init() -> Context {
    // let job_queue = Rc::new(job_queue::TokioJobQueue::new());
    // let mut context = Context::builder().job_queue(job_queue).build().unwrap();
    let mut context = Context::default();

    let console = Console::init(&mut context);

    context
        .register_global_property(Console::NAME, console, Attribute::all())
        .expect("the console builtin shouldn't exist");

    context
        .register_global_builtin_callable(
            "say_hello".into(),
            1,
            NativeFunction::from_fn_ptr(say_hello),
        )
        .unwrap();

    context
        .register_global_builtin_callable("sleep".into(), 1, NativeFunction::from_async_fn(sleep))
        .unwrap();

    context
}

fn sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> impl Future<Output = JsResult<JsValue>> {
    let duration = args.get_or_undefined(0);
    let duration = duration.to_number(context).unwrap();

    async move {
        println!("Delaying for {duration} seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(duration as u64)).await;
        println!("Done!");

        Ok(JsValue::undefined())
    }
}

fn say_hello(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = args.get_or_undefined(0);

    if name.is_undefined() {
        println!("Hello World!");
    } else {
        // println!("Hello {}!", name.to_string(context)?);
        println!(
            "Hello {}!",
            name.to_string(context)?.to_std_string_escaped()
        );
    }

    Ok(JsValue::undefined())
}
