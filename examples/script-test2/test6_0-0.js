const max = 10000;
const iter = 100000000;

async function fetch(input) {
    let count = 0;
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }
    await sleep(1000);
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }

    console.log("test6 done");
    return "";
}

export default fetch;
