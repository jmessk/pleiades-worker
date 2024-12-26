async function fetch(input) {
    // console.log("http-post.js");

    let post_data = new TextEncoder().encode("example_data");

    let client = new HttpClient();
    let response = await client.post("https://example.com", post_data);

    let body = new TextDecoder().decode(response);
    console.log(body);

    return "test_output";
}

export default fetch
