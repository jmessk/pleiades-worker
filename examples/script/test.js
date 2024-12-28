async function fetch(input) {
    console.log("1");
    syncSleep(200);
    console.log("2");
    await sleep(200);
    console.log("3");
    syncSleep(200);
    console.log("4");
    await sleep(200);
    console.log("5");
    // await yieldNow();

    return "test_output";
}

export default fetch;
