# Link a pre-compiled component into an Ark consumer

Demonstrates **Ark ↔ compiled component** interop:

| Role | Artifact | Source |
|------|----------|--------|
| Provider (implements `test:host/math`) | `provider.component.wasm` | `provider.wit` — packaged with official `wasm-tools` |
| Consumer socket (imports `test:host/math`) | `client.ark` → `client.component.wasm` | This directory |

`client.ark` uses WIT package import syntax:

```ark
import "test:host/math" as host

pub fn run() -> i32 {
    host::add(40, 2)
}
```

`ark.toml` vendors the WIT package under `vendor/host/`. After both sides are compiled to
components, `arukellt compose --validate` and `wac plug` link them. The provider is a
standard WIT-based component placeholder; execution belongs to the host that supplies
the provider implementation.

## Alternative: Ark provider

You can also export the provider from Ark (see `tests/component-interop/compose/math_lib.ark`)
and plug it into a **func-import** socket (`import add: func(...)`). The example here uses
**interface import**, which matches real WIT packages shared across languages.

## Run

```bash
bash examples/ark/link-compiled/run.sh
```

Requires: `wasm-tools`, `wac`, and a selfhost compiler wasm with WIT import support
(`.build/selfhost/arukellt-s2.wasm`, or set `ARUKELLT_SELFHOST_WASM`).
