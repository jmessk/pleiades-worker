async function fetch(input) {
    // console.log("counter-sleep.js");

    let counter = 0;
    for (let i = 0; i < 1000000; i++) {
        counter++;
    }

    await sleep(100);
}

export default fetch;
