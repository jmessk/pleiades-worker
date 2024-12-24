import { blob } from "pleiades"

async function fetch(input) {
    console.log("get-blob.js");

    for (let i = 0; i < 10; i++) {
        await blob.get("0");
    }

    return "test_output";
}

export default fetch
