// fib.rs — Rust reference implementation for the Arukellt fib benchmark.
// Matches benchmarks/fib.ark: iterative Fibonacci(35).
// Compile: rustc -O -o fib fib.rs
// Run:     ./fib

fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    let (mut a, mut b) = (0i32, 1i32);
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}

fn main() {
    println!("{}", fib(35));
}
