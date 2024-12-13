function say_hello_a(name) {
  say_hello("Hello " + name);
}

async function say_hello_p(name) {
  say_hello("Hello " + name);
}

say_hello("World");
say_hello_a("sub");
say_hello();

new Promise(async (resolve) => {
  say_hello("Promise");
  await say_hello_p("Promise await");
  resolve();
});
