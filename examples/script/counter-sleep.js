async function fetch(input) {
    // console.log("counter-sleep.js");

    await sleep(50);

    let counter = 0;
    for (let i = 0; i < 1000000; i++) {
        counter++;
    }

}

export default fetch;
