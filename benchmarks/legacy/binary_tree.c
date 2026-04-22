/* binary_tree.c — C reference implementation for the Arukellt binary_tree
 * benchmark.  Matches benchmarks/binary_tree.ark: count nodes (depth 20).
 * Compile: cc -O2 -o binary_tree binary_tree.c
 * Run:     ./binary_tree
 */
#include <stdio.h>

static int count_nodes(int depth) {
    if (depth == 0)
        return 1;
    return 1 + count_nodes(depth - 1) + count_nodes(depth - 1);
}

int main(void) {
    printf("%d\n", count_nodes(20));
    return 0;
}
