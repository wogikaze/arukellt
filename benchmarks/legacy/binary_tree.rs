// binary_tree.rs — Rust reference implementation for the Arukellt
// binary_tree benchmark.  Matches benchmarks/binary_tree.ark: count nodes
// (depth 20).
// Compile: rustc -O -o binary_tree binary_tree.rs
// Run:     ./binary_tree

fn count_nodes(depth: i32) -> i32 {
    if depth == 0 {
        return 1;
    }
    1 + count_nodes(depth - 1) + count_nodes(depth - 1)
}

fn main() {
    println!("{}", count_nodes(20));
}
