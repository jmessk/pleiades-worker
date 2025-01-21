const max = 20000;
const iter = 150000000;

async function fetch(input) {
    busy(iter * 0.5);
    blockingSleep(1000);
    busy(iter * 0.5);

    console.log("test6_o done");
    return "";
}

export default fetch;
