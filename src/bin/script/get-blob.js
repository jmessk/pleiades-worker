import { blob } from "pleiades"

async function fetch(input) {
    const input = await blob.get("0");

    console.log("got blob");

    return "test_output"
}

export default fetch
