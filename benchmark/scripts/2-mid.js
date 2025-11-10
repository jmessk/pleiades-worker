export default async function fetch(input) {
    const iter = 8000;
    let state = 123456789;
    
    for (let i = 0; i < iter; i++) {
        state = (state * 1664525 + 1013904223) & 0xffffffff;
    }

    return state;
}
