async function fetch(input) {
    // console.log("counter.js started");

    let counter = 0;
    for (let i = 0; i < 150000000; i++) {
        counter++;
    }
    // let counter = fib(48);

    // console.log("counter.js finished");
    return "output";
}

export default fetch;
