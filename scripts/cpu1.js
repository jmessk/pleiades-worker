export default async function fetch(input) {
    console.log("hello from cpu1.js");

    const client = new HttpClient();

    // const response = await client.get("https://example.com");
    const response = client.getSync("https://example.com");

    return "";
}
