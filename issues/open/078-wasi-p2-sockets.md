# WASI P2: wasi:sockets TCP/UDP ネイティブバインディング

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 078
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

## Summary

WASI Preview 2 の `wasi:sockets/tcp`・`wasi:sockets/udp`・`wasi:sockets/ip-name-lookup` を
`std/sockets` モジュールとして Arukellt に追加する。
resource 型 (`tcp-socket`, `udp-socket`) の lifecycle (create → bind → listen/connect → accept → io → close)
を canonical ABI で実装する。

## 受け入れ条件

1. `std/sockets` に `TcpListener` / `TcpStream` / `UdpSocket` 型を追加
2. `wasi:sockets/tcp.{create-tcp-socket, bind, connect, accept, receive, send, close}` を呼ぶ
3. fixture: `tcp_echo_server.ark` が wasmtime-net で接続可能なエコーサーバを起動

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §wasi:sockets
