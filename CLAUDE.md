# 開発方針＆開発環境ルール(open-shop)

一般個人から貿易商社まで出品・出店可能な総合通販(構想段階、Rust)。
最終的には高級家具から格安家具、自動車・バイク・航空機まで新品/中古品の
売買、保険代理店連携、世界中の宅配便と連携した国際発送・貿易対応を目指す。
複数フェーズに分割して段階的に実装する方針(ユーザーとの合意事項)。

エコシステム共通の開発ルール(Rust中心、実機検証必須、正直な開示徹底、
新規フォルダ作成前の相談ルール、DB非依存・1バイナリ完結・テンプレート
エンジン不使用等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`が
**正本**であり、詳細をここに複製せずそちらを参照する。

## 構成

```
open-shop/
├── Cargo.toml (workspace)
├── crates/
│   ├── shop-core/   # データモデル(Seller/Product/Cart)
│   └── shop-api/    # tokio/hyper直接実装のHTTP API
```

## フェーズ計画(ユーザー指定)

1. **コアEC基盤(第1フェーズ、本セッションで実装済み)**: 出品者/出店者
   モデル、商品モデル、出品API、商品検索API、カートAPI(注文確定・決済は
   含まない)。
2. **高額・中古品出品フロー**: 自動車・バイク・航空機の出品項目
   (車検証・走行距離等)、注文確定・決済処理(Stripe連携等)。
3. **保険代理店API連携**。
4. **国際発送・貿易対応**(世界中の宅配便との連携)。

**第1フェーズでは2〜4の項目には一切着手していない**(ユーザーの明示的な
スコープ指定に従い、決済・保険・国際発送関連のコードは1行も書いていない)。

## 設計方針

- `open-fudousan/crates/fudousan-api/src/main.rs`(直近セッションでテスト
  可能な純粋関数へのルーティング分離・丁寧なエラーメッセージへ改善済み)を
  読み、同じ設計パターン(hyperの`Incoming`に依存しない同期の純粋関数
  `route(method, path, query, body, state)`へルーティングを分離、
  クエリ文字列の簡易パーサー(`parse_query`/`simple_url_decode`)を流用、
  具体的なフィールド名・有効値を含むエラーメッセージ)を踏襲した。
- `fudousan-api`との違い: open-shopは出品・カート追加等の書き込みAPIを
  持つため、`AppState`はハッシュマップ+`Mutex`で保持する(fudousan-apiは
  ダミーデータの読み取り専用だったため`Vec`のみで足りていた)。

## HANDOFF

- **2026-08-21 新規作成・第1フェーズ実装完了**:
  - `shop-core`: `Seller`(`SellerKind::Individual`/`Business`で個人/法人
    出店者を区別)、`Product`(`Category`は家具の7区分の簡易列挙、
    `Condition::New`/`Used`)、`Cart`(`add_item`/`remove_item`/
    `total_jpy`)を実装。`Cart`に5件の単体テストを追加(重複追加時の
    数量加算、削除、カタログから消えた商品の合計金額での無視、等)。
  - `shop-api`: `POST /v1/sellers`・`POST /v1/products`・
    `GET /v1/products/search`・`GET /v1/products/:id`・
    `POST /v1/carts`・`GET /v1/carts/:id`・`POST /v1/carts/:id/items`・
    `DELETE /v1/carts/:id/items/:product_id`・`GET /healthz`を実装。
    16件の単体テスト(出品者登録・商品登録の妥当性検証(存在しない
    seller_id・不正なcategory/condition)・検索フィルタの複合条件・
    カートのadd/remove/合計金額フロー・404/400/405の各レスポンス)を
    追加。
  - **検証結果**: `cargo build --release`成功(警告なし)。
    `cargo test --release`で`shop-core` 5件+`shop-api` 16件、
    全21件成功。実サーバー(`SHOP_PORT`未指定時8081番)を起動し、
    PowerShellの`Invoke-RestMethod`で実HTTPリクエストにより
    出品者登録→商品登録→検索→カート作成→追加→合計金額確認
    (48000円×2個=96000円で一致)→削除→合計金額0円に戻ることを確認、
    存在しない商品IDへの404も確認。
  - **スコープ外(意図的に未実装、次フェーズ)**: 注文確定・決済処理
    (Stripe連携含む)、高額・中古品特有の出品項目(車検証・走行距離等)、
    保険代理店API連携、国際発送・貿易・宅配便API連携、認証・認可、
    ページネーション、実DB永続化(現状プロセス内インメモリのみ、
    再起動でデータ消失)、画像の実ファイルアップロード(URL文字列のみ)。
  - 次にすべきこと: 第2フェーズ(高額・中古品出品フロー、決済処理)に
    着手する場合は、まず決済プロバイダ(Stripe等)の選定と、
    `open-fudousan`のHANDOFFにある「aruaru-dbへの永続化」検討状況を
    確認してから進めること(open-shopも同様にプロセス内インメモリの
    ままでは実運用に耐えないため)。
