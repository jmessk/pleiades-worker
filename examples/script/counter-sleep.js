async function fetch(input) {
    // console.log("counter-sleep.js");

    await sleep(50);
    let counter = 0;
    for (let i = 0; i < 3000000; i++) {
        counter++;
    }
    await sleep(50);
    for (let i = 0; i < 3000000; i++) {
        counter++;
    }
    await sleep(50);
}

export default fetch;
