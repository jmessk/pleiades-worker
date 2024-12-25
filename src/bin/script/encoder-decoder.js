import { blob } from "pleiades"

async function fetch(input) {
    console.log("encoder-decoder.js");

    let decoded_input = new TextDecoder().decode(input);
    console.log(decoded_input);

    let output = new TextEncoder().encode("test_output")
    console.log(output);

    return output;
}

export default fetch
