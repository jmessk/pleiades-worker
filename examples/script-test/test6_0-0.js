const max = 20000;
const iter = 150000000;

async function fetch(input) {
    count(iter);
    await sleep(1000);
    count(iter);

    console.log("test6 done");
    return "";
}

export default fetch;
