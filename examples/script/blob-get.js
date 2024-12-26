import { blob } from "pleiades"

async function fetch(input) {
    console.log("get-blob.js");
    let data = await blob.get("0");

    return "test_output";
}

export default fetch
