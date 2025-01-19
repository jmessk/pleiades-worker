const max = 5000;

async function fetch(input) {
    busy(max * 0.5);
    await sleep(1000);
    busy(max * 0.5);

    console.log("test6 done");
    return "";
}

export default fetch;
