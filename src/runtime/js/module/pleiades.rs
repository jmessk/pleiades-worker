use boa_engine::{context, Module, Source};

const JS_CODE: &str = r#"
    const blob = new Blob();

    export { blob };
"#;

pub fn get_module(context: &mut context::Context) -> Module {
    Module::parse(Source::from_bytes(JS_CODE), None, context).unwrap()
}
