import { blob } from "pleiades";

async function fetch(input) {
    console.log("counter-sleep.js");


    await sleep(100);

    let counter = 0;
    for (let i = 0; i < 100000; i++) {
        counter++;
    }

    await sleep(100);

    return counter;
}

export default fetch;
