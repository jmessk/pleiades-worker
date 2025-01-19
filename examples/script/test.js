import { blob } from "pleiades"

async function fetch(input) {
    // console.log("http-get.js");
    // let data = await blob.get("0");

    await sleep(200);

    let counter = 0;
    for (let i = 0; i < 10000000; i++) {
        counter++;
    }

    // let client = new HttpClient();
    // let response = await client.post("http://pleiades.local:8000/upload", data);

    // let body = new TextDecoder().decode(response.toIntU8Array());
    // console.log(body);

    return "test_output";
}

export default fetch
