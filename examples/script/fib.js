async function fetch(input) {
    function fib(n) {
        if (n < 2) {
            return n;
        }

        return fib(n - 1) + fib(n - 2);
    }
    let n = fib(29);
    return "test_output";
}

export default fetch;
