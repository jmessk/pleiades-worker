export default async function fetch(input) {

    // const client = new HttpClient();
    // const response1 = await client.get("http://localhost/");
    let iter = 20000;
    let state = 123456789;

    for (let i = 0; i < iter; i++) {
        state = (state * 1664525 + 1013904223) & 0xffffffff;
    }

    return state;

    // const client = new HttpClient();
    // const response2 = await client.get("http://localhost/");

    return "";
}
