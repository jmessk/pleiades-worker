const max = 20000;
const iter = 150000000;

async function fetch(input) {
    let count = 0;
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }
    blockingSleep(1000);
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }

    console.log("test6_o done");
    return "";
}

export default fetch;
