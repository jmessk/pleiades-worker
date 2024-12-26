async function fetch(input) {
    // console.log("http-get.js");
    
    let client = new HttpClient();
    let response = await client.get("https://example.com");

    let body = new TextDecoder().decode(response);
    console.log(body);

    return "test_output";
}

export default fetch
