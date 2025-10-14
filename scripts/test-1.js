export default async function fetch(input) {

    let count = 0;
    for (let i = 0; i < 10000; i++) {
        count += i;
    }

    const client = new HttpClient();
    const response = await client.get("http://localhost/");

    for (let i = 0; i < 10000; i++) {
        count += i;
    }

    return "";
}
