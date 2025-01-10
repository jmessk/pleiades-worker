async function fetch(input) {
    // console.log("counter.js");

    let counter = 0;

    for (let i = 0; i < 5000000; i++) {
        counter++;
    }

    return counter;
}

export default fetch;
