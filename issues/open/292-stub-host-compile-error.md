# stub host module (http, sockets) の使用を compile error にする

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 292
**Depends on**: —
**Track**: capability
**Blocks v1 exit**: no
**Priority**: 12

## Summary

`std/host/http.ark` と `std/host/sockets.ark` は error stub を返すだけの実装。使用者はコンパイルは通るが実行時に常にエラーになる。未実装 module の使用を compile-time に検出して error にすべき。

## Current state

- `std/host/http.ark`: `request()` / `get()` が `Err("not implemented")` を返す
- `std/host/sockets.ark`: `connect()` が `Err("not implemented")` を返す
- `std/manifest.toml:1685-1720`: `kind = "host_stub"` で分類済み

## Acceptance

- [ ] `kind = "host_stub"` の関数を呼び出すコードが compile error (E レベル) を出す
- [ ] error メッセージに「この API は未実装です (host_stub)」と表示される
- [ ] `std/manifest.toml` の `kind` 情報がコンパイラに伝搬する経路がある
- [ ] テスト: http::get を呼ぶコードが compile error を出す fixture

## References

- `std/host/http.ark`
- `std/host/sockets.ark`
- `std/manifest.toml:1685-1720`
