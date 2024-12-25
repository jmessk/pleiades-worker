import { blob } from "pleiades";

async function fetch(input) {
    console.log("counter-pend.js");

    let counter = 0;

    for (let i = 0; i < 100000; i++) {
        counter++;
    }

    let data = await blob.get("0");

    return counter;
}

export default fetch;
