import { blob } from "pleiades";

async function fetch(input) {
    let inputId = new TextDecoder().decode(input);
    let inputBlob = await blob.get(inputId);

    let output = compress(inputBlob);

    return output;
}

export default fetch;
