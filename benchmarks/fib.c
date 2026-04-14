/* fib.c — C reference implementation for the Arukellt fib benchmark.
 * Matches benchmarks/fib.ark: iterative Fibonacci(35).
 * Compile: cc -O2 -o fib fib.c
 * Run:     ./fib
 */
#include <stdio.h>

static int fib(int n) {
    if (n <= 1)
        return n;
    int a = 0, b = 1;
    for (int i = 2; i <= n; i++) {
        int next = a + b;
        a = b;
        b = next;
    }
    return b;
}

int main(void) {
    printf("%d\n", fib(35));
    return 0;
}
