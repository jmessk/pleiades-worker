export default async function fetch(input) {

    let count = 0;
    for (let i = 0; i < 100000; i++) {
        count += i;
    }

    // console.log("hello from cpu1.js");

    // const client = new HttpClient();

    // const response = await client.get("https://example.com");
    // const response = client.getSync("https://example.com");

    return "";
}
