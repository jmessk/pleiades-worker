import { blob } from "pleiades"

async function fetch(input) {
    console.log("get-blob.js");

    let data = await blob.get("0");

    // for (let i = 0; i < 10; i++) {
    //     await blob.get("0");
    // }

    console.log("got blob");

    return "test_output";
}

export default fetch
