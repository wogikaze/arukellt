# RFC-008: native-cpp C99 backend と runtime ABI

ステータス: ACCEPTED
関連 ADR: [ADR-049](../adr/ADR-049-native-c99-selfhost-executor.md)
関連 plan: [native-cpp MVP implementation plan](../plans/native-cpp-mvp-implementation.md)
Capability SSOT: [`data/native-cpp-capabilities.toml`](../../data/native-cpp-capabilities.toml)
日付: 2026-07-22

---

## 1. 適用範囲と正本

本 RFC は、experimental なセルフホスト executor に限定した `native-cpp` の C99
出力、private ABI、runtime、MIR lowering、cache、receipt を規定する。
採用理由と非目標は ADR-049、実装順は implementation plan、個々の MIR opcode と
CoreOp の状態は capability registry が所有する。

現行 target は scaffold である。
本 RFC の `ACCEPTED` は実装済みまたは実行可能を意味しない。

## 2. Target と artifact

canonical target 名は当面 `native-cpp` とする。
生成言語は portable C99、既定拡張子は `.c` とする。
`-o` を省略したときは入力 path の `.ark` を `.c` に置換し、`.ark` 以外なら `.c` を追加する。

MVP の driver は MIR から単一の C source を `output_bytes` として返す。
object、executable、runtime 結合、cache、receipt は manager が管理し、driver API を
複数 artifact 用へ拡張しない。
将来 `--emit c` を追加するときも、この単一 C source 契約を使用する。

runtime header と runtime C は独立した正本ファイルとする。
巨大な runtime source を Ark の文字列 literal として手書きしない。
MVP は manager が repository 内の runtime source を clang へ直接渡すため、runtime の
generated Ark resource を作らない。

clang は shell command string ではなく argv 配列で起動する。
入力 path、出力 path、flag、environment override を shell 展開しない。

## 3. Toolchain 契約

基準 toolchain は clang 14.0.0 以上とする。
manager は候補 compiler に `--version`を argv で渡し、先頭の semantic version を取得する。
version を解析できない場合または 14.0.0 未満の場合は toolchain diagnostic とする。

compiler の探索順は次とする。

1. `ARUKELLT_CC` の明示 path
2. `PATH` 上の `clang`

`ARUKELLT_CC` は実行ファイル path 一つだけを受け取る。
flag や shell fragment を含む値は拒否する。

release の必須引数は `-std=c99 -O2`、debug は `-std=c99 -O0 -g` とする。
generated C の検証では両モードに `-Wall -Wextra -Wpedantic -Werror` を追加する。
`-march=native`、LTO、`-ffast-math`は禁止する。

clang がない場合は toolchain diagnostic とする。
clang が generated C を拒否した場合は backend/toolchain failure とし、exit code、stderr、
C source の SHA-256、保存 path を診断へ含める。
失敗した C source は `.build/selfhost/native-cpp/failures/<sha256>.c` に保存する。
debug mode では成功時も generated C を `.build/selfhost/native-cpp/debug/` に保持する。

## 4. Scalar ABI

scalar の C 表現を次に固定する。

| Ark 型 | C 型 |
|--------|------|
| `i8` | `int8_t` |
| `i16` | `int16_t` |
| `i32` | `int32_t` |
| `i64` | `int64_t` |
| `u8` | `uint8_t` |
| `u16` | `uint16_t` |
| `u32` | `uint32_t` |
| `u64` | `uint64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `bool` | `uint8_t`。値は 0 または 1 |
| `char` | `uint32_t` |
| unit return | `void` |
| unit value | `ark_unit`。値は常に 0 |
| reference | 対象 object の typed pointer |
| nullable reference | C null pointer を許す typed pointer |

unit value は次の定義を使う。

```c
typedef uint8_t ark_unit;
#define ARK_UNIT ((ark_unit)0)
```

`ark_unit` は field、tuple、generic argument、複数戻り値に使う。
戻り値が unit だけの関数は `void` を返す。

MIR の `MirValueType.repr` が scalar C 型を決める。
backend は source type 名から narrow width や signedness を推測しない。
subword 型の情報が MIR まで保持されない場合は backend で補わず、typed MIR owner の欠落として
ICE にする。

## 5. Integer と浮動小数点の意味論

native-cpp は既存 `wasm32-gc` backend の scalar 意味論を再現する。
C の未定義動作または実装依存動作を Ark の意味論として利用しない。

### 5.1 Integer

- signed add、sub、mul の wrap は同じ幅の unsigned 型へ変換して演算し、bit pattern を戻す。
- divide by zero と signed `INT_MIN / -1` は演算前に runtime trap とする。
- signed remainder の `INT_MIN % -1` は既存 Wasm 規則どおり 0 とする。
- i32 shift count は `((uint32_t)rhs) & 31u`、i64 は `((uint64_t)rhs) & 63u` とする。
- 範囲外または負の shift count を独自に trap にしない。
- logical right shift は unsigned 値を shift する。
- arithmetic right shift は sign bit を明示的に補い、C の負数右 shift に依存しない。
- narrow conversion は切り捨てる bit と sign extension または zero extension を明示する。
- allocation size の加算、乗算、`uint32_t` と `size_t` の変換は事前に overflow を検査する。

C の関数引数と複合式の評価順には依存しない。
各 MIR 命令を独立した statement と temporary へ変換する。

### 5.2 Floating point

`float` と `double` は IEEE 754 binary32 と binary64 であることを toolchain probe で確認する。
確認できない target は本 RFC の対象外とする。

- NaN 入力と NaN 結果は既存 Wasm operation の分類と比較規則を再現する。
- NaN payload は既存 Wasm 契約が固定しない箇所で新たに固定しない。
- signed zero を保持し、`-0.0` を `+0.0` へ正規化しない。
- infinity の arithmetic と comparison は既存 Wasm operation を再現する。
- float-to-int は NaN、infinity、範囲外を明示検査してから C cast を行う。
- rounding は既存 Wasm operation の round-to-nearest ties-to-even その他の命令別規則を再現する。
- executor は floating-point environment を変更しない。

未固定の浮動小数点意味論を native 側で決めない。
Wasm 実装と文書が食い違う場合は Wasm 実装を再現し、ドリフトを別 issue 候補として記録する。

## 6. Object header と arena

すべての heap object は次の header で始まる。

```c
typedef struct {
    uint32_t type_id;
    uint32_t flags;
} ark_object_header;
```

`type_id` は TypeTable が所有する決定的 ID とする。
C declaration order、pointer address、link order、collection iteration orderから生成しない。
`flags` は MVP では全 bit を予約し、object 作成時に 0 を書く。

null は C null pointer だけで表す。
TypeId 0、空 object、zero-length allocation を null sentinel に使わない。
object address はプロセス終了まで移動しない。

arena は 16-byte 境界へ切り上げた chunk から割り当てる。
要求 alignment が 16 byte 以下なら要求値と 16 の大きい方を使う。
MVP が 16 byte を超える alignment の型を受け取った場合は capability diagnostic とする。
chunk の拡張前に要求 size と alignment padding の overflow を検査する。

arena は allocation bytes、allocation count、chunk count を記録する。
個別 free と phase reset は提供しない。

## 7. String layout

String は次の layout を使う。

```c
typedef struct {
    ark_object_header header;
    uint8_t *bytes;
    uint32_t byte_length;
    uint32_t capacity;
} ark_string;
```

`byte_length` と `capacity` は byte 単位である。
`byte_length <= capacity` を常に満たす。
capacity が 0 の場合に限り `bytes` は null でよい。

NUL 終端を言語意味論へ含めない。
runtime が libc 呼び出し用の補助 NUL を確保する場合、その byte は capacity 外に置き、
`byte_length`へ含めない。
embedded NUL は String data として保持する。

> native-cpp は各 String CoreOp について、現行 wasm32-gc 実装と同じ index 単位、
> 戻り値、invalid UTF-8 処理を再現する。
> Unicode 意味論の変更は native-cpp のスコープ外とする。

String source は UTF-8 byte 列または既存 runtime が許す byte 列として保持する。
`len`、`char_at`、slice の単位を native backend 独自に変更しない。
言語文書と Wasm 実装の既存ドリフトは本 RFC で解決しない。

## 8. Array、Vec、slice

### 8.1 Array

array は header、length、typed flexible array member の一 allocation に固定する。

```c
typedef struct {
    ark_object_header header;
    uint32_t length;
    ark_element data[];
} ark_array_element;
```

上の型は模式表現である。
実際には element TypeId ごとに `ark_array_<mangled_type>` を生成する。
inline と external buffer を選択可能にしない。

allocation size は `offsetof(ark_array_T, data) + length * sizeof(T)` とし、積と加算を
個別に overflow check する。
data の alignment は element 型と arena の 16-byte 規則の両方を満たす。

### 8.2 Vec

Vec は element TypeId ごとの typed buffer を使う。

```c
typedef struct {
    ark_object_header header;
    ark_element *data;
    uint32_t length;
    uint32_t capacity;
} ark_vec_element;
```

実際の型名は `ark_vec_<mangled_type>` とする。
万能 `uintptr_t` buffer は使用しない。
`length <= capacity` を常に満たし、growth 後も element alignment を維持する。
get、set、remove、slice 作成は bounds check を行う。

arena は旧 buffer を解放しない。
Vec growth は新しい typed buffer を確保して有効要素を copy し、Vec の data pointer を更新する。
reference element の copy は pointer copy、value element の copy はその型の value copy 規則に従う。

### 8.3 Slice

slice は heap object ではない pair とする。

```c
typedef struct {
    ark_element *data;
    uint32_t length;
} ark_slice_element;
```

MVP arena は backing storage をプロセス終了まで保持するため、slice に独立した owner field を
追加しない。
native 独自の borrow、lifetime、解放規則を公開しない。

## 9. Aggregate と Any

### 9.1 Struct と tuple

struct と tuple は header の後に TypeTable canonical field order で field を置く。
C compiler は Linux x86-64 LP64 の自然 alignment に従って field 間へ padding を入れる。
backend は `offsetof` と `sizeof` を生成 C と runtime の内部でだけ使用し、その値を Wasm の
TypeId、field 順、出力順へ伝播させない。

alignment は各 field の C 型 alignment の最大値とし、object 先頭は arena の 16-byte
alignment を満たす。
field orderを padding 最適化のために並べ替えない。

### 9.2 Enum、Option、Result

enum は header、`uint32_t` tag、payload union の順とする。
tag は TypeTable の canonical variant order を 0 始まりで割り当てる。
payload のない variant も固有 tag を持つ non-null object とする。

```c
typedef struct {
    ark_object_header header;
    uint32_t tag;
    union {
        ark_variant_0 variant_0;
        ark_variant_1 variant_1;
    } payload;
} ark_enum_example;
```

Option と Result は同じ一般規則を使う。
active tag 以外の union field を読まない。
constructor は active payload 全体を初期化する。

複数戻り値は signature ごとに生成した C struct を値で返す。
複数値を整数へ pack しない。

### 9.3 Any

Any は object header pointer 一つで表す。

```c
typedef ark_object_header *ark_any;
```

heap object は pointer をそのまま格納する。
primitive は型ごとの box object へ格納する。

```c
typedef struct {
    ark_object_header header;
    int32_t value;
} ark_box_i32;
```

他の primitive も同じ `header + typed value` 規則を使う。
type test は `ark_any->type_id` を読み、unbox は TypeId 一致を確認してから typed pointer へ
castする。
nullable Any は null pointer とする。
Any 自身に重複した TypeId field を追加しない。

## 10. Function ABI

### 10.1 Symbol mangling

C function symbol は次の文法で生成する。

```text
ark_f_<FunctionId>__P_<parameter-type-encoding>__R_<result-type-encoding>
```

FunctionId は符号なし十進表記とする。
type encoding は `i8`、`i16`、`i32`、`i64`、`u8`、`u16`、`u32`、`u64`、`f32`、`f64`、
`b`、`c`、`v`、`r<TypeId>`、`n<TypeId>` を使用する。
複数 parameter と result は underscore で連結する。
monomorphized aggregate は concrete TypeId を使う。

同一 module 内で同じ C symbol が異なる FunctionId または signature から生成された場合は
emit 前の ICE とする。
ユーザー識別子は diagnostic comment にだけ使い、C identifierへ直接埋め込まない。

### 10.2 Parameter と return

scalar、unit value、slice、closure、複数戻り値 struct は C value で渡す。
heap object は typed pointer で渡す。
unit だけを返す関数は `void` とする。
複数戻り値は signature ごとの generated struct return とする。

direct call は mangle 済み C function を呼ぶ。
indirect call は完全 signature ごとの typed function pointer を使う。
C の可変長引数と untyped `void (*)()` cast は使用しない。

closure は signature ごとの function pointer と environment pointer の pair とする。

```c
typedef struct {
    ark_closure_fn_signature function;
    ark_object_header *environment;
} ark_closure_signature;
```

environment は通常の typed heap object とし、captured field は TypeTable canonical order を使う。
generic は MIR 生成前の monomorphization 結果だけを受け取る。
trait dispatch は MIR に解決済みの FunctionId または vtable identity を使う。

### 10.3 Global と entry point

module global は canonical module identity と declaration identity で安定 sortして宣言する。
C static initializationで表せない値は `ark_module_init_<ModuleId>` へ分離する。
module initializer は依存 module を先にした canonical module order で一度だけ実行する。

C entry point は次の adapter を持つ。

```c
int main(int argc, char **argv);
```

adapter は arena と runtime を初期化し、argv[0] を除いた args view を登録し、module initializerを
実行して Ark entry pointを呼ぶ。
Ark main が通常 return した場合は 0、`process.exit` は指定 code、panic は非 0 を返す。
runtime abort は Linux の SIGABRT 慣例に従う 134 を process result とする。

## 11. MIR CFG lowering

MIR から C へ直接 lowerし、新しい汎用 BackendIR を作らない。
basic block は C label、branch は `goto`、conditional branch は `if` と `goto`、
`br_table` は `switch` とする。

local と MIR temporary は関数冒頭で宣言する。
各 block 内では宣言済み local への代入だけを行う。
VLA は生成しない。
structured C の loop や nested if への復元は行わない。

phi は predecessor edge ごとに parallel copy として lowerする。
source と destination が循環する場合は型付き scratch temporary を使い、後続代入で上書きされる
値を先に保存する。

MIR opcode の規則を次に分類する。
個々の状態と phase は capability registry を正とする。

| 規則 | MIR opcode | C lowering |
|------|------------|------------|
| `mir-constant` | `MIR_CONST_*` | typed literalまたはString object constructor |
| `mir-local` | `MIR_LOCAL_GET`, `MIR_LOCAL_SET` | C local readまたはassignment |
| `mir-arithmetic` | arithmetic、comparison、logical、bit、shift、neg、not、eqz | §5のsequenced statement |
| `mir-direct-call` | `MIR_CALL`, `MIR_RETURN` | typed direct callまたはC return |
| `mir-indirect-call` | `MIR_CALL_INDIRECT`, `MIR_REF_FUNC` | signature別function pointerまたはclosure |
| `mir-cast` | extend、trunc、ref test、ref cast、cast branch | checked scalar conversionまたはTypeId check |
| `mir-cfg` | branch、table、block marker、phi | label、goto、switch、edge copy |
| `mir-aggregate` | struct、array、GC struct operation | §6から§9のtyped object operation |
| `mir-effect` | drop、nop、unreachable、GC hint | sequenced discard、no statement、trap、非観測hint消費 |
| `mir-unsupported` | WIT call、future、await | emit前target capability diagnostic |

`MIR_BLOCK`、`MIR_LOOP`、`MIR_IF`、`MIR_ELSE`、`MIR_END` は検証済みCFGのmarkerとして消費し、
それ自体からstructured Cを生成しない。
`MIR_GC_HINT` と `MIR_GC_HINT_SHORT_LIVED` は観測可能な意味を持たないhintとして消費し、
arena lifetimeを短縮しない。

全 opcode は [`data/native-cpp-capabilities.toml`](../../data/native-cpp-capabilities.toml) に一度だけ
登録する。
`Unsupported` entry は理由を持ち、emit前 validation で拒否する。

## 12. Runtime ABI

runtime ABI version は `uint32_t` 定数 `ARK_NATIVE_RUNTIME_ABI_VERSION` とする。
generated C は compile 時に期待 version と header version の一致をC99 typedef assertionで検査する。
manager cache keyも同じ versionを含める。

### 12.1 Error boundary

host runtime の回復可能な操作は `int32_t` status を返す。
0 は成功、正値は次の分類とする。

| 値 | 定数 | 意味 |
|----|------|------|
| 0 | `ARK_RT_OK` | 成功 |
| 1 | `ARK_RT_NOT_FOUND` | pathまたはenvironment keyなし |
| 2 | `ARK_RT_PERMISSION_DENIED` | permission拒否 |
| 3 | `ARK_RT_INVALID_UTF8` | UTF-8変換失敗 |
| 4 | `ARK_RT_IO_ERROR` | その他のI/O失敗 |
| 5 | `ARK_RT_OUT_OF_MEMORY` | allocation失敗 |

回復可能な operation は最後の `ark_string **error_out` に所有されたerror Stringを返す。
generated adapter はstatusを既存のArk `Result`、`Option`、bool契約へ変換する。
runtimeがArk enum layoutを名前から推測しない。

### 12.2 Allocation とobject check

| C symbol | Parameter | Return | 契約 |
|----------|-----------|--------|------|
| `ark_rt_arena_init` | runtime state、initial size | status | chunked arenaを初期化 |
| `ark_rt_alloc_aligned` | `size_t size`, `size_t alignment` | `void *` | overflowとOOMを検査。失敗はtrap boundaryへ送る |
| `ark_rt_alloc_object` | size、alignment、TypeId | `ark_object_header *` | headerを初期化しflagsを0にする |
| `ark_rt_check_non_null` | object、span id | `void` | nullならruntime trap |
| `ark_rt_check_type` | object、expected TypeId、span id | `void` | 不一致ならruntime trap |
| `ark_rt_check_bounds` | index、length、span id | `void` | `0 <= index < length`を検査 |
| `ark_rt_trap` | trap kind、span id | 戻らない | Ark trap messageと非0終了 |
| `ark_rt_panic` | message | 戻らない | Ark panicをstderrへ出して非0終了 |

allocation counterは要求byte、実際のaligned byte、allocation count、chunk countを分けて保持する。

### 12.3 String とVec raw operation

| C symbol | Parameter | Return | Ownershipと失敗 |
|----------|-----------|--------|-----------------|
| `ark_rt_string_new` | なし | `ark_string *` | arena所有の空String |
| `ark_rt_string_from_bytes` | bytes、length | `ark_string *` | byteをarenaへcopy。length overflowはtrap |
| `ark_rt_string_clone` | String | `ark_string *` | byteを新規arena storageへcopy |
| `ark_rt_string_byte_at` | String、index | `uint8_t` | bounds check後に返す |
| `ark_rt_string_compare` | lhs、rhs | `int32_t` | bytewise orderingを返す |
| `ark_rt_vec_grow_T` | typed Vec、capacity | status | 新bufferをarenaへ確保してcopy |
| `ark_rt_array_copy_T` | typed dst/srcと範囲 | `void` | 範囲とoverlapを検査したtyped copy |

`T`を含むsymbolはTypeIdのmangleを含むgenerated helperとする。
高水準String、Vec、parse、format、semantic stdlibはArk fallbackを優先する。

### 12.4 Host operation

host ABI は次のraw operationを持つ。

| C symbol | Parameter | Returnとownership |
|----------|-----------|-------------------|
| `ark_rt_fs_read_bytes` | path、`Vec<u8> **out`、error | status。成功値はarena所有 |
| `ark_rt_fs_read_string` | path、`ark_string **out`、error | status。invalid UTF-8を区別 |
| `ark_rt_fs_write_bytes` | path、bytes、error | status。全byte完了までwrite |
| `ark_rt_fs_write_string` | path、String、error | status。NUL終端へ依存しない |
| `ark_rt_fs_readable_file` | path、`uint8_t *out` | status。現行existsのread-probe意味論 |
| `ark_rt_args_count` | なし | `uint32_t`。argv[0]を除く |
| `ark_rt_args_at` | index、String out | status。範囲外はNOT_FOUND |
| `ark_rt_env_get` | key、String out、error | OK、NOT_FOUND、その他error |
| `ark_rt_stdin_read_all` | String out、error | status。EOFまで読む |
| `ark_rt_stdout_write` | bytes、length、error | status。partial writeを内部で完了 |
| `ark_rt_stderr_write` | bytes、length、error | status。partial writeを内部で完了 |
| `ark_rt_process_exit` | `int32_t code` | 戻らない |
| `ark_rt_process_abort` | なし | 戻らない。process resultは134 |
| `ark_rt_clock_monotonic_ns` | なし | `int64_t` nanoseconds |
| `ark_rt_clock_wall_ms` | なし | `int64_t` Unix epoch milliseconds |

pathとenvironment keyはUTF-8 Stringからbyte sequenceへ変換し、embedded NULを拒否する。
filesystem errorは`ENOENT`をNOT_FOUND、`EACCES`と`EPERM`をPERMISSION_DENIEDへ写像し、
その他をIO_ERRORへ写像する。
`EINTR`は再試行する。

readとwriteはpartial resultを内部loopで処理する。
読み込みbyte数が`UINT32_MAX`を超える、または必要allocationを`size_t`で表せない場合は
IO_ERRORとしてlarge-file messageを返す。
UTF-8 String readは全byte取得後に検証し、不正列をINVALID_UTF8にする。

`print`はstdout raw write、`println`はStringとnewline、`eprintln`はStringとnewlineを順に書く
generated adapterとする。
高水準Ark APIのerror型は既存stdlib契約を維持する。

## 13. CoreOp boundary とcapability

CoreOpIdの正本は [`data/core-ops.toml`](../../data/core-ops.toml) とする。
native固有の状態は [`data/native-cpp-capabilities.toml`](../../data/native-cpp-capabilities.toml)
が所有する。
callee名の文字列比較をbackendへ追加しない。

backend固有実装はprimitive、runtime ABI、host operation、target intrinsic、機械的C loweringへ
限定する。
semantic stdlibはArk fallbackを優先する。

capability registryのschemaは次の規則を持つ。

- `supported`は`implementation`を必須とする。
- `planned`は`phase`と`implementation`を必須とする。
- `unsupported`は`reason`を必須とする。
- 全MIR opcodeと全CoreOpを一度だけ含め、unknown IDとduplicate IDを拒否する。
- targetがscaffoldの間は実装前entryを`Supported`にしない。

emit前validatorはregistryとMIRを照合し、Unsupportedをtarget capability diagnosticとして
non-zero exitにする。
scaffold、stub、偽値、無条件`unreachable`を成功として扱わない。

## 14. Cache key

native executable cache keyは次を長さ付きbyte sequenceとして連結し、SHA-256を計算する。

- s2 compiler artifact SHA-256とcompiler source fingerprint
- native runtime source SHA-256とruntime ABI version
- clangのcanonical pathと完全な`clang --version`出力
- compile flags、link flags、Linux x86-64 target triple
- backend schema versionとcapability registry schema/content SHA-256

source fingerprintは`src/compiler/`の対象file pathとcontent SHA-256をcanonical path順に
連結して作る。
pointer address、一時path、wall clock、process ID、unordered iteration順をcache keyに含めない。
cacheは正本ではなく、削除しても生成結果を変えてはならない。

## 15. Performance receipt

managerはnative executor pipelineごとにmachine-readable JSON receiptを生成する。
receipt schema versionを整数で持ち、最低限次のfieldを含める。

| Field | 型 | 測定対象 |
|-------|----|----------|
| `clang_peak_rss_bytes` | integer | generated Cのcompileとlink |
| `executor_peak_rss_bytes` | integer | `arukellt-native`によるS3生成 |
| `pipeline_peak_rss_bytes` | integer | manager command全体 |
| `executor_wall_time_ms` | integer | warm S3生成のみ |
| `pipeline_wall_time_ms` | integer | cache missを含む全体 |
| `s2_sha256` | lowercase hex string | 比較元 |
| `s3_sha256` | lowercase hex string | native executor生成物 |
| `determinism_run_1_sha256` | lowercase hex string | 一回目 |
| `determinism_run_2_sha256` | lowercase hex string | 二回目 |
| `clang_version` | string | toolchain識別 |
| `runtime_abi_version` | integer | runtime ABI識別 |
| `cache_hit` | boolean | native executable cache結果 |
| `exit_code` | integer | pipeline結果 |

2.4 GiB gateは`executor_peak_rss_bytes`だけへ適用する。
`pipeline_peak_rss_bytes`は記録するが、別processであるclang peakをexecutor上限へ合算しない。

warm 5分未満の区間は、cache済み`arukellt-native`を起動してから`s3.wasm`のcloseが成功するまで
とする。
性能計測方法とthreshold変更はbenchmark governanceに従う。

## 16. Determinism と safety

generated C、native executable cache identity、native executorが生成するWasmは決定的な入力を
使う。

- mapとsetの未規定iteration順を出力順へ使わない。
- pointer address、clock、environment、一時pathをsymbolまたはartifactへ埋め込まない。
- C layout順とclang link順をWasm emitterのTypeIdまたは出力順へ伝播させない。
- identifierは§10のmangleだけを使う。
- string literalのquote、backslash、NUL、非ASCIIをC escapeまたはbyte arrayへ変換する。

同じnative executableと入力でS3を二回生成し、両hashとs2 hashの一致を検査する。

## 17. Diagnostic

| 問題 | 分類 | 必須context |
|------|------|-------------|
| 未対応機能 | target capability diagnostic | opcodeまたはCoreOpId、理由、span |
| 壊れたMIR | compiler ICE | FunctionId、block、instruction、span |
| clang不在または旧version | toolchain diagnostic | 探索path、要求version |
| clang compile失敗 | backend/toolchain failure | exit、stderr、C path、C hash |
| runtime panic | Ark panic | message、non-zero exit |
| s2/s3不一致 | selfhost verification failure | 両hash、receipt path |

generated Cはfunction定義の直前にFunctionId、完全signature、元Ark spanをcommentとして出す。
emitterは将来`#line`を追加できるよう、source spanをC writerへ渡す。
ユーザー入力から到達するUnsupportedをICEまたはruntime trapへ遅延させない。

## 18. 現行ドリフトの扱い

Stringのindex単位と`char_at`説明には、byte-orientedな実装とcode point表記のドリフトがある。
native-cppは現行`wasm32-gc`実装を再現し、Unicode意味論を変更しない。

CoreOp registryにはmigration中のlegacy signatureとhandlerが残る。
native backendはそのplaceholder signatureからABIを推測せず、typed MIRとSignatureRegistryを使う。

## 関連

- [ADR-002: GC vs non-GC](../adr/ADR-002-memory-model.md)
- [ADR-006: 公開 ABI 境界の分類](../adr/ADR-006-abi-policy.md)
- [ADR-029: セルフホストネイティブ検証契約](../adr/ADR-029-selfhost-native-verification-contract.md)
- [ADR-040: Semantic Type Spine](../adr/ADR-040-typed-mir-signature-registry.md)
- [ADR-042: Intrinsic Layer Separation](../adr/ADR-042-intrinsic-layer-separation.md)
- [ADR-049: Native C99 Selfhost Executor](../adr/ADR-049-native-c99-selfhost-executor.md)
- [RFC-006: Sealed raw API](006-sealed-raw-api.md)
- [native-cpp MVP implementation plan](../plans/native-cpp-mvp-implementation.md)
