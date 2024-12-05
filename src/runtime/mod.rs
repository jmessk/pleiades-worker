pub mod runtime_thread;
pub mod js;
pub mod python;

pub trait Context {}

pub trait Runtime {
    fn init(&mut self);
    fn run(&mut self, input: &str);
}
