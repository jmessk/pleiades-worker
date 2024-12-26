import { blob } from "pleiades"

async function fetch(input) {
    // console.log("pend-sleep.js");

    let data = await blob.get("0");
    await sleep(1000);

    console.log("got blob");

    return "test_output";
}

export default fetch
