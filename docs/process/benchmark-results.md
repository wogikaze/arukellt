# Benchmark Results — T1 linear vs T3 Wasm GC (#686 Phase 4)

T1 (linear memory) と T3 (Wasm GC) の benchmark 結果の記録先と比較サマリ。
測定は 2026-10-08、selfhost s2 コンパイラ（bootstrap overlay で gc_hint pass が `-O2` に有効化）、
`python3 scripts/util/benchmark_runner.py --mode full --target <t>`、
5 compile iterations + 5 runtime iterations + 1 warmup、median 値。

## Canonical reports and payloads

- Generated reports: `../history/benchmarks/benchmark-results.md`（runner が再生成する正本）、
  `../history/benchmarks/benchmark-686-wasm32.md`（T1 full run）、
  `../history/benchmarks/benchmark-686-wasm32-gc.md`（T3 full run）。
- Prior quick-mode recordings: `../history/benchmarks/benchmark-879-wasm32.md`,
  `../history/benchmarks/benchmark-879-wasm32-gc.md`。
- Machine-readable payloads（schema `arukellt-bench-v1`）:
  `tests/baselines/perf/current-wasm32.json`（T1, mode=full）、
  `tests/baselines/perf/current-wasm32-gc.json`（T3, mode=full）、
  `tests/baselines/perf/current.json`（compare モードの既定出力）、
  `tests/baselines/perf/baselines.json`（回帰 baseline、wasm32）。
- Reproduce: `python3 scripts/util/benchmark_runner.py --mode full`（T1）、
  `--mode full --target wasm32-gc --output-json tests/baselines/perf/current-wasm32-gc.json`（T3）、
  `--mode compare`（baseline 回帰比較）。

## T1 vs T3 summary (median, 2026-10-08)

| Workload | T1 compile ms | T3 compile ms | T1 run ms | T3 run ms | T1 bytes | T3 bytes |
|----------|--------------:|--------------:|----------:|----------:|---------:|---------:|
| fib | 18060.227 | 8508.897 | 11.614 | 14.815 | 2284 | 10880 |
| binary_tree | 13573.369 | 8620.951 | 196.123 | 18.822 | 2302 | 10899 |
| vec_ops | 11316.855 | 8659.375 | 10.655 | 19.55 | 3735 | 12571 |
| string_concat | 12133.548 | 8485.066 | 15.697 | 14.952 | 2563 | 11428 |
| parse_tree_distance | 8726.148 | 8344.473 | 452.275 | 95.592 | 12801 | 18203 |
| enum_dispatch | 6015.398 | 8589.441 | 18.968 | 16.336 | 3917 | 12109 |
| struct_graph | 6369.907 | 8231.444 | 51.405 | 15.979 | 4360 | 12659 |
| error_chain | 6229.644 | 8382.303 | 21.053 | 18.418 | 4724 | 12816 |
| closure_map | 6043.47 | 8392.507 | 7.251 | 44.295 | 4232 | 12340 |
| http_parser | 6022.826 | 7218.152 | 11.593 | 13.768 | 4013 | 12389 |
| log_processor | 6292.002 | 7075.222 | 7.976 | 16.265 | 4418 | 12770 |
| config_loader | 6062.757 | 7082.925 | 10.217 | 14.721 | 5000 | 13249 |
| data_pipeline | 6252.829 | 7105.607 | 7.893 | 20.074 | 5672 | 13513 |
| template_engine | 6273.534 | 7226.523 | 8.33 | 22.97 | 17465 | 23169 |
| file_io | 6166.295 | 7077.924 | 12.63 | 26.279 | 6880 | 17468 |
| json_parse | 6106.523 | 7251.178 | 12.663 | 34.4 | 60320 | 55860 |
| text_ops | 6184.279 | 7118.599 | 7.745 | 24.477 | 10355 | 18791 |
| hash_ops | 7901.32 | 8230.988 | 9.011 | 21.892 | 19367 | 26244 |
| toml_ops | 6513.225 | 7713.528 | 13.68 | 41.809 | 85699 | 74134 |
| time_ops | 6338.9 | 6999.397 | 9.504 | 14.136 | 3940 | 12703 |

## Reading the comparison

- Run time: T3 は allocation-heavy workload（binary_tree 196→19ms、
  parse_tree_distance 452→96ms、struct_graph 51→16ms）で有意に速い。
  scalar / app loop 系（closure_map、file_io、toml_ops、time_ops）は T1 が速い。
  T3 は 20/20 workload が runtime pass。T1 では allocation-heavy workload が
  tracing GC metadata arena の制約で遅いか trap することがある（#896）。
- Compile time: T3 は component packaging 分だけ常時 +0.6〜1.2s。
  T1 の先頭 workload（fib 18.1s）は cold-cache ウォームアップを含む。
- Binary size: T3 component は約 8KB の envelope を持つため小さな
  fixture では 4〜6 倍（fib 2284→10880B）。stdlib-heavy workload では
  T3 の方が小さい場合がある（json_parse 60320→55860B、toml_ops 85699→74134B）。
- Variance: 共有ホスト計測のため runtime CV が 5% を超える workload が複数ある
  （file_io 218% など）。傾向比較は median の方向のみを根拠とし、
  閾値 gate の判断材料にはしない（#879 と同じ注意）。
- GC pause / allocation-rate telemetry は wasmtime CLI から取得不可のため
  `"unavailable"` 記録。

## Provenance

- Compiler: `scripts/run/arukellt-selfhost.sh` → `.build/selfhost/arukellt-s2-runtime.wasm`
  （bootstrap overlay で gc_hint pass が `-O2` に有効化された build）。
- Environment: wasmtime 46.0.1, wasm-tools, Linux
  6.18.33.2-microsoft-standard-WSL2 x86_64（WSL2 共有ホスト）。
- 測定 JSON は compile/run samples、CV、RSS、binary_bytes を保持し、
  T1/T3 を同一 schema で比較可能。
