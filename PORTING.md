# PORTING

現時点(2026-08-21新規作成)では移植元・移植先の対応関係は無い
(新規リポジトリとして設計・実装したもの)。

- APIエンドポイント形状(`GET /v1/xxx?param=value`)・エラーレスポンス
  設計(具体的なフィールド名・有効値を含むメッセージ)・ルーティングの
  純粋関数分離パターンは、`open-fudousan`の`crates/fudousan-api/src/main.rs`
  (直近セッションでテスト可能な純粋関数へのルーティング分離・丁寧な
  エラーメッセージへ改善済み)を直接読み、一貫したスタイルとなるよう
  踏襲した。クエリ文字列パーサー(`parse_query`/`simple_url_decode`)も
  同一実装を流用している。
- `open-fudousan`との構造的な違い: open-shopは出品・カート追加等の
  書き込みAPIを持つため`AppState`が`Mutex<HashMap<..>>`による可変状態
  を持つのに対し、`open-fudousan`はダミーデータの読み取り専用APIの
  ため`Vec`のみで完結している。
- エコシステム共通方針(DB非依存・1バイナリ完結・テンプレートエンジン
  不使用・hyper直接実装)は`open-raid-z/CLAUDE.md`に準拠。
