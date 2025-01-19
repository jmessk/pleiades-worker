import { blob } from "pleiades";

async function fetch(input) {
    let inputId = new TextDecoder().decode(input);
    let inputBlob = await blob.get(inputId);

    let output = compress(inputBlob);
    
    let client = new HttpClient();
    let response = await client.post("http://pleiades.local:8000/upload", output);

    return output;
}

export default fetch;
