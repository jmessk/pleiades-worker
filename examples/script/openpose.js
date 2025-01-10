import { blob, ai } from "pleiades"

async function fetch(input) {
    let inputId = new TextDecoder().decode(input);
    let inputBlob = await blob.get(inputId);

    let output = await ai.infer("openpose", inputBlob);

    return "test_output";
}

export default fetch
