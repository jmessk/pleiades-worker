use boa_engine::{Context, JsArgs, JsResult, JsValue, Source, NativeFunction};

fn main() -> JsResult<()> {
  let js_code = r#"
    say_hello("World");
    say_hello();
  "#;

  // Instantiate the execution context
  let mut context = Context::default();
  context.register_global_builtin_callable("say_hello".into(), 1, NativeFunction::from_fn_ptr(say_hello))?;


  let result = context.eval(Source::from_bytes(js_code))?;
  println!("{}", result.display());

  Ok(())
}

fn say_hello(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = args.get_or_undefined(0);

    if name.is_undefined() {
        println!("Hello World!");
    } else {
        // println!("Hello {}!", name.to_string(context)?);
        println!("Hello {}!", name.to_string(context)?.to_std_string_escaped());
    }

    Ok(JsValue::undefined())
}
