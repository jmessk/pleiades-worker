const iter = 150000000;
const duration = 10000;

async function fetch(input) {
    let count = 0;
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }
    blockingSleep(1000);
    for (let i = 0; i < iter / 2; i++) {
        count++;
    }

    return "";
}

export default fetch;
