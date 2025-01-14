import { blob } from "pleiades"

async function fetch(input) {
    console.log("get-blob-10.js");

    let iteration = 10;

    for (let i = 0; i < iteration; i++) {
        console.log("iter", i);
        await blob.get("0");
    }

    return "test_output";
}

export default fetch
