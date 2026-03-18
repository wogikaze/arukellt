Go は goroutine と channel を標準に組み込み、CSP の発想で共有メモリより通信を推奨した。
Go は並行性を問題分割の構造として扱い、並列性を実行機構として分離した。
Go の M:N スケジューラは goroutine を OS スレッドより軽量に運用できるように設計された。
Go の設計は多数接続サーバで大量タスクを扱いやすくし、スレッド数管理の負担を下げた。
Go のスケジューラはプリエンプションとワークスティーリングを取り込み、スループットと公平性を改善してきた。
Go では goroutine の低コスト化と引き換えに、リークや starvation の診断が難しい課題が残る。
Erlang/OTP は軽量プロセスとメッセージパッシングを中核にし、let it crash と監視ツリーで耐障害性を優先した。
Erlang/OTP は電話交換機のような停止不能システム向けに、プロセス隔離とホットコード更新を重視した。
Erlang/OTP の supervisor tree と gen_server は耐障害分散システムの定番パターンとして定着した。
Erlang の関数型スタイルは起源の影響が大きく、アクターモデル自体が関数型を必須にするわけではない。
Erlang では不変データとアクターモデルの組み合わせが並行バグを減らす実践として広く受容された。
Elixir は BEAM の並行性と耐障害性を継承しつつ、モダンな構文とマクロで開発体験を改善した。
Elixir は OTP を DSL 的に扱いやすくし、Phoenix などの Web 開発でアクターモデル活用を広げた。
Rust は std::sync::mpsc::channel を標準に置き、スレッド間メッセージパッシングの基本を提供した。
Rust の std::sync::mpsc は所有権モデルと整合するように multi-producer single-consumer の制約を採用した。
Rust の標準 mpsc は単純パイプラインに適する一方、複雑トポロジでは crossbeam や tokio::sync::mpsc が使われやすい。
Pony はアクターモデルに reference capabilities を組み合わせ、参照エイリアス可能性を型で表現した。
Pony の reference capabilities は共有経由のデータレースをコンパイル時に排除する狙いで導入された。
Pony は実行時オーバーヘッドを抑えるため、ケイパビリティ検査を主にコンパイル時に行う。
Pony の iso と consume は一意所有の移譲を明示し、コピーなしの安全なアクター間転送を可能にする。
Swift は Concurrency Manifesto の流れで async/await と actor を導入し、非同期制御を構文で明示した。
Swift の async/await はコールバック中心の記述を減らし、既存コードを段階的に移行しやすくした。
Swift の actor は共有可変状態を隔離し、アクセスを await 境界で管理してデータレースを抑制する。
C# は async/await を導入し、継続渡し中心の非同期 I/O を同期風の制御フローで記述可能にした。
C# の async/await は Task ベース非同期パターンの普及を後押しし、例外処理とリソース解放を自然化した。
C# の async/await モデルは JavaScript、Kotlin、Swift など後続言語の設計に強い影響を与えた。
Rust の async/await はゼロコスト抽象化方針に従い、Future の状態機械化と Pin による移動制約を採用した。
Rust の Pin 設計は自己参照 future の安全性を確保するために必要とされた。
Rust の非同期基盤は利用者側を簡潔に保つ一方で、ライブラリ作者には Pin、Poll、Unsafe の複雑性を要求する。
Python は PEP 3156 で asyncio を導入し、イベントループと Future/Task を標準化した。
Python の asyncio は Twisted や Tornado など分散していた非同期流儀の収斂を狙って設計された。
Python では PEP 492 以降の async/await が asyncio 基盤と結合し、標準非同期スタックが確立した。
JavaScript は Promise を先に導入し、その後 async/await を重ねて callback hell の複雑性を下げた。
JavaScript の async/await はエラーハンドリングを簡潔化したが、Promise API との相互運用設計を継続的に要した。
Kotlin は coroutines と structured concurrency を標準化し、親子ジョブ単位でキャンセルと例外伝搬を扱う。
Kotlin の coroutineScope は子失敗を全体に波及させ、supervisorScope は失敗分離を選べる。
Swift は structured concurrency で TaskGroup と async let を導入し、タスク階層を明示化した。
Swift の TaskGroup は動的並列、async let は少数の静的並列に向く。
What Color is Your Function? 問題に対して Rust、Swift、Kotlin は async/await の着色を受け入れつつ増殖制御を設計した。
async main や TaskGroup や nursery の導入は非同期色の伝播をエントリポイントとスコープで吸収する戦略である。
Rust の fearless concurrency は Send と Sync でスレッド間送受信可能性と共有可能性を型に刻む。
Rust では Rc や RefCell のような Send/Sync 非対応型はスレッド越し利用を禁止される。
Rust で unsafe による Send/Sync 実装を行う場合はライブラリ作者が強い不変条件を保証する責任を負う。
Java は Project Loom で Virtual Thread を導入し、既存 Thread API の互換性を保ちながら軽量スレッド化した。
Java の Virtual Thread はブロッキングスタイルの可読性を維持しつつ高並行性を狙う設計である。
Java の Loom は従来のスレッド数上限問題を緩和し、Executor とブロッキング API の運用を改善した。
Python の GIL は CPython の単純性と参照カウント整合性を優先して導入された。
Python の GIL は CPU バウンドなマルチスレッド並列を阻害し、マルチプロセスや C 拡張依存を強めた。
Python は PEP 703 で free-threaded CPython を提案し、GIL なし実行に向けた内部刷新を進めた。
Python 3.13 の GIL-less ビルドは実験段階で、単一スレッド性能低下と並列性能向上のトレードオフを持つ。
Ruby の MRI は GVL により Ruby バイトコード実行を実質単一スレッド化する設計を採った。
Ruby は並列性補完のために Ractor を導入し、共有制限とメッセージパッシング中心のモデルを追加した。
Ruby の Ractor は真の並列実行を提供する一方で、既存ライブラリ適合と API 制約の課題を伴う。
Structured Concurrency は fire-and-forget を抑制し、タスク生成と join を同一スコープに閉じる原則を採る。
Structured Concurrency の目的はエラー伝搬、キャンセル、リソース回収を親子タスク木で一貫化することにある。
Trio、Kotlin、Swift、Java は構造化並行性 API を採用し、タスクリークを防ぐ方向へ収束した。
Python Trio の nursery は start_soon したタスクがスコープ終了前に完了することを保証する。
Python Trio の nursery は例外を親へ再送し、勝手なバックグラウンド化を抑えて因果性を保つ。
Java 21 の StructuredTaskScope は関連タスクを一単位で join と結果集約する API を提供する。
Java 21 の StructuredTaskScope は shutdown-on-failure などで高水準なキャンセル方針を表現できる。
Julia は多重ディスパッチを中核に据え、型に応じた SIMD、マルチスレッド、分散実装の選択を可能にした。
Julia の設計は高水準 API を維持したまま BLAS や GPU バックエンドへの最適化分岐を実現する。
Mojo は Python 互換構文と SIMD や並列ループ制御を統合し、HPC/ML 向け性能を狙う。
Mojo は parallelize などでスレッド並列とベクトル並列を同一言語層で扱える設計を採る。
Futhark は純関数データ並列言語として map、reduce、scan を中核に据える。
Futhark は OpenCL や GPU カーネル生成の詳細をコンパイラに隠蔽し、自動並列化を重視する。
Futhark コンパイラは高水準記述を GPU 向けカーネル群とホストコードへ変換して最適化する。
Rust の std::simd 構想は fearless SIMD を掲げ、unsafe を減らした型安全なベクトル演算を目指す。
Rust の std::simd は intrinsics 依存を緩和し、安全性と性能の両立を狙う。
Rust の std::simd は API 形状の議論を継続しつつ安定化へ向けて進んでいる。
Java の Thread.stop はロック保持中停止で不整合を生みうるため非推奨となった。
Java は Thread.stop から interrupt と Future/Executor による協調的キャンセルへ移行した。
Go は fire-and-forget 的な go 文により、キャンセル漏れで goroutine リークやチャネルデッドロックを起こしやすい。
Go では context.Context と errgroup が構造化並行性を補う実務パターンとして広く使われる。
Rust の async/await はランタイム非同梱方針により Tokio と async-std などのエコシステム分断を生みやすい。
Node.js は初期のコールバック中心設計で制御フローとエラー処理の可読性を大きく損ねた。
Node.js は Promise と async/await の導入で callback hell を段階的に改善した。
C++ の std::thread は join と detach の管理責務が重く、低レベル過ぎるという批判を受ける。
C++ では std::thread の上位抽象として std::async や Boost.Asio や独自スレッドプールの利用が一般化した。
