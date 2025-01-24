async function fetch(input) {
    console.log("counter.js");

    let counter = 0;
    for (let i = 0; i < 100000000; i++) {
        counter++;
    }

    console.log("counter.js finished");
    return counter;
}

export default fetch;
