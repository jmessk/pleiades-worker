import { blob } from "pleiades";

async function fetch(input) {
    console.log("counter-sleep.js");


    await sleep(200);

    let counter = 0;
    for (let i = 0; i < 10000; i++) {
        counter++;
    }

    await sleep(200);

    return counter;
}

export default fetch;
