async function fetch(input) {
    syncSleep(50);
    await sleep(50);
    syncSleep(50);
    await sleep(50);
    // await yieldNow();

    return "test_output";
}

export default fetch;
