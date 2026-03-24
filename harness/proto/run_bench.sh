#!/bin/bash
# Benchmark runner for GC vs linear memory comparison
# Requires: wasmtime with GC support, wat2wasm

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GC_DIR="$SCRIPT_DIR/gc"
LINEAR_DIR="$SCRIPT_DIR/linear"
RESULTS_FILE="$SCRIPT_DIR/results.txt"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check dependencies
check_deps() {
    if ! command -v wasmtime &> /dev/null; then
        echo -e "${RED}Error: wasmtime not found${NC}"
        exit 1
    fi
    if ! command -v wat2wasm &> /dev/null; then
        echo -e "${YELLOW}Warning: wat2wasm not found, trying wasm-tools${NC}"
        if ! command -v wasm-tools &> /dev/null; then
            echo -e "${RED}Error: neither wat2wasm nor wasm-tools found${NC}"
            exit 1
        fi
        WAT_COMPILER="wasm-tools parse"
    else
        WAT_COMPILER="wat2wasm"
    fi
}

# Compile WAT to WASM
compile_wat() {
    local wat_file=$1
    local wasm_file="${wat_file%.wat}.wasm"
    
    if [[ "$WAT_COMPILER" == "wasm-tools parse" ]]; then
        wasm-tools parse "$wat_file" -o "$wasm_file" 2>/dev/null
    else
        wat2wasm "$wat_file" -o "$wasm_file" 2>/dev/null
    fi
    
    echo "$wasm_file"
}

# Measure binary size
measure_size() {
    local wasm_file=$1
    stat --printf="%s" "$wasm_file" 2>/dev/null || stat -f%z "$wasm_file" 2>/dev/null
}

# Run benchmark
run_bench() {
    local wasm_file=$1
    local gc_enabled=$2
    local iterations=${3:-10}
    
    local total_time=0
    local times=()
    
    for ((i=1; i<=iterations; i++)); do
        local start_ns=$(date +%s%N)
        if [[ "$gc_enabled" == "true" ]]; then
            wasmtime run --wasm gc "$wasm_file" > /dev/null 2>&1
        else
            wasmtime run "$wasm_file" > /dev/null 2>&1
        fi
        local end_ns=$(date +%s%N)
        local elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
        times+=($elapsed_ms)
    done
    
    # Calculate median
    IFS=$'\n' sorted=($(sort -n <<<"${times[*]}")); unset IFS
    local median=${sorted[$((iterations/2))]}
    echo $median
}

# Print result comparison
print_comparison() {
    local test_name=$1
    local gc_size=$2
    local linear_size=$3
    local gc_time=$4
    local linear_time=$5
    
    echo ""
    echo "=== $test_name ==="
    echo ""
    printf "%-20s %12s %12s %12s\n" "Metric" "GC" "Linear" "Ratio"
    printf "%-20s %12s %12s %12s\n" "-------------------" "------------" "------------" "------------"
    
    if [[ -n "$gc_size" && -n "$linear_size" ]]; then
        local size_ratio=$(echo "scale=2; $gc_size / $linear_size" | bc)
        printf "%-20s %10d B %10d B %11sx\n" "Binary Size" "$gc_size" "$linear_size" "$size_ratio"
    fi
    
    if [[ -n "$gc_time" && -n "$linear_time" && "$linear_time" -gt 0 ]]; then
        local time_ratio=$(echo "scale=2; $gc_time / $linear_time" | bc)
        printf "%-20s %9d ms %9d ms %11sx\n" "Execution Time" "$gc_time" "$linear_time" "$time_ratio"
    fi
}

# Main benchmark function
run_all_benchmarks() {
    echo "========================================"
    echo "  Arukellt Benchmark: GC vs Linear"
    echo "========================================"
    echo ""
    echo "Date: $(date)"
    echo "Wasmtime: $(wasmtime --version)"
    echo ""
    
    # Test cases
    local tests=("hello" "vec_pushpop")
    
    for test in "${tests[@]}"; do
        local gc_wat="$GC_DIR/${test}.wat"
        local linear_wat="$LINEAR_DIR/${test}.wat"
        
        local gc_size="" gc_time=""
        local linear_size="" linear_time=""
        
        # GC version
        if [[ -f "$gc_wat" ]]; then
            echo -n "Compiling GC/${test}.wat... "
            if gc_wasm=$(compile_wat "$gc_wat" 2>&1); then
                echo -e "${GREEN}OK${NC}"
                gc_size=$(measure_size "$gc_wasm")
                echo -n "Running GC/${test} (10 iterations)... "
                gc_time=$(run_bench "$gc_wasm" "true" 10)
                echo -e "${GREEN}${gc_time}ms${NC}"
            else
                echo -e "${RED}FAILED${NC}"
                echo "  (GC types may not be supported by wat2wasm)"
            fi
        fi
        
        # Linear version
        if [[ -f "$linear_wat" ]]; then
            echo -n "Compiling Linear/${test}.wat... "
            if linear_wasm=$(compile_wat "$linear_wat" 2>&1); then
                echo -e "${GREEN}OK${NC}"
                linear_size=$(measure_size "$linear_wasm")
                echo -n "Running Linear/${test} (10 iterations)... "
                linear_time=$(run_bench "$linear_wasm" "false" 10)
                echo -e "${GREEN}${linear_time}ms${NC}"
            else
                echo -e "${RED}FAILED${NC}"
            fi
        fi
        
        # Print comparison
        print_comparison "$test" "$gc_size" "$linear_size" "$gc_time" "$linear_time"
    done
    
    echo ""
    echo "========================================"
    echo "  Benchmark Complete"
    echo "========================================"
}

# Quick mode: just compile and verify
quick_check() {
    echo "Quick check: verifying WAT files compile..."
    
    for dir in "$GC_DIR" "$LINEAR_DIR"; do
        local label=$(basename "$dir")
        for wat in "$dir"/*.wat; do
            [[ -f "$wat" ]] || continue
            local name=$(basename "$wat")
            echo -n "  $label/$name: "
            if compile_wat "$wat" > /dev/null 2>&1; then
                echo -e "${GREEN}OK${NC}"
            else
                echo -e "${RED}FAILED${NC}"
            fi
        done
    done
}

# Parse arguments
check_deps

case "${1:-}" in
    --quick)
        quick_check
        ;;
    *)
        run_all_benchmarks | tee "$RESULTS_FILE"
        echo ""
        echo "Results saved to: $RESULTS_FILE"
        ;;
esac
