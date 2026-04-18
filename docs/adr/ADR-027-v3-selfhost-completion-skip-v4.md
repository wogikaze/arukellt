# ADR-027: v3完了時点でのセルフホスト完了とv4スキップ

## Status

Accepted (2026-04-19)

## Context

当初のroadmapでは以下の順序で進行する計画でした：
- v1: Wasm GCネイティブ対応 (完了 2026-03-27)
- v2: Component Model対応 (完了 2026-03-28)
- v3: 標準ライブラリ整備 (完了)
- v4: 最適化 (未着手)
- v5: セルフホスト (未着手)

しかし、以下の状況から方針転換が必要になりました：
1. v1/v2/v3の技術的実装は完了しているが、製品リリース基準を満たしていないissuesが「Blocks v1 exit」として残っている
2. v4（最適化）は性能改善だが、言語機能やエコシステム自立性には直接寄与しない
3. セルフホスト（Rust→Arukellt完全移行）はエコシステム自立性の観点から優先度が高い
4. v3までの実装が部分的に進んでしまっており、ここを完璧にしてから次に進むべき

## Decision

**v4（最適化）をスキップし、v3完了時点でセルフホストを完了させる**

### 具体的な方針

1. **v4の実装をスキップ**
   - `roadmap-v4.md`に記載された最適化パスは実装しない
   - 将来的に性能問題が顕在化した場合に再評価する

2. **v3完了時点でセルフホスト完了**
   - v3（stdlib整備）完了時点で、RustコンパイラをArukelltで完全に再実装する
   - Stage 0 (Rust版) → Stage 1 (Arukellt版) → Stage 2 (fixpoint) の達成
   - Rust実装は参照実装として保持するが、開発はArukellt版に移行する

3. **v3完了条件の明確化**
   - 技術的実装の完了だけでなく、製品リリース基準を満たすこと
   - テスト戦略、selfhost parity、VS Code拡張E2Eなどの品質面を含める

4. **v4以降の開発はArukelltのみで行う**
   - v3完了後はRustコードの新規開発は原則停止
   - バグ修正はRust版とArukellt版の両方に適用（dual period継続中）
   - fixpoint達成後はArukellt版がcanonical implementationとなる

## Rationale

### なぜv4をスキップするのか

1. **優先順位**: セルフホスト（エコシステム自立性） > 最適化（性能改善）
2. **依存関係**: 最適化はselfhostコンパイラの性能に寄与するが、selfhost自体の達成には必須ではない
3. **再評価可能性**: 性能問題が顕在化した時点でv4を再開できる

### なぜv3完了時点でselfhostを完了するのか

1. **stdlib完了**: v3でstdlibが整備されているため、selfhostに必要な機能が揃っている
2. **技術的基盤**: v1/v2でGC-native T3とComponent Modelが完了しており、selfhostの基盤が整っている
3. **エコシステム自立**: Rust依存を減らし、Arukelltエコシステムで閉じた開発を可能にする

### なぜv3を完璧にするのか

1. **リリース品質**: 技術的実装だけでなく、製品としての品質を担保する
2. **selfhostの前提**: テスト戦略やCI構造が整っていないと、selfhostの検証が困難
3. **技術的負債回避**: 不完全な状態でselfhostに進むと、二重実装の負債が増大する

## Consequences

### Positive

1. エコシステム自立性が早期に達成される
2. Rust依存が減り、Arukelltでの開発が自立する
3. v4（最適化）は将来的に必要になった時点で再開できる

### Negative

1. 性能改善が遅れる（最適化パスが実装されない）
2. v3完了までの期間が長くなる可能性がある（品質面の基準を満たす必要があるため）
3. selfhost達成までのdual期間中、Rust版とArukellt版の両方をメンテする必要がある

### Mitigation

1. 性能問題が顕在化した場合はv4を再開する
2. v3完了条件を明確にし、進捗を可視化する
3. dual期間中のメンテ負担を軽減するため、共通テストやCI構造を整備する

## Alternatives Considered

1. **当初のroadmap通りv4→v5と進む**
   - 拒否理由: セルフホストの優先度が高いこと、v4は必須ではないこと

2. **v3とv4を並行して進める**
   - 拒否理由: リソースが分散する、v4の成果がselfhostに直結しない

3. **v3完了前にselfhostを開始する**
   - 拒否理由: 品質面の基準が整っていないとselfhostの検証が困難

## References

- [roadmap-v3.md](../process/roadmap-v3.md) — stdlib整備完了
- [roadmap-v4.md](../process/roadmap-v4.md) — 最適化（スキップ）
- [roadmap-v5.md](../process/roadmap-v5.md) — セルフホスト（v3完了時点で達成）
- [docs/compiler/bootstrap.md](../compiler/bootstrap.md) — セルフホスト手順
- [docs/release-criteria.md](../release-criteria.md) — リリース基準
