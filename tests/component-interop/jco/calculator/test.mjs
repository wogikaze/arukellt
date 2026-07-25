// Component interop test fixture: jco-transpiled scalar assertions.
//
// Imports JS glue from `jco transpile` and asserts calculator s32 exports.
// Library components use real i32 (not Memory64 i64), so Node Number values
// match the WIT s32 canonical ABI.

import { add, mul, negate } from './jco-out/calculator.component.js';

let pass = 0;
let fail = 0;

function assert_eq(desc, actual, expected) {
    if (actual === expected) {
        console.log(`      PASS: ${desc}`);
        pass += 1;
    } else {
        console.log(`      FAIL: ${desc} — expected '${expected}', got '${actual}'`);
        fail += 1;
    }
}

console.log('[1/2] Asserting scalar exports');
assert_eq('add(3, 4) = 7', add(3, 4), 7);
assert_eq('add(0, 0) = 0', add(0, 0), 0);
assert_eq('add(-1, 1) = 0', add(-1, 1), 0);
assert_eq('mul(6, 7) = 42', mul(6, 7), 42);
assert_eq('mul(0, 100) = 0', mul(0, 100), 0);
assert_eq('negate(5) = -5', negate(5), -5);
assert_eq('negate(-3) = 3', negate(-3), 3);

console.log(`[2/2] Results: ${pass} passed, ${fail} failed`);
process.exit(fail > 0 ? 1 : 0);
