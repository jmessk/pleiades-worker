async function fetch(input) {
    syncSleep(200);
    await sleep(200);
    syncSleep(200);
    // await yieldNow();

    return "test_output";
}

export default fetch;
