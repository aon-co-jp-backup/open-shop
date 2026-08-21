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

- **2026-08-21(続き) 第2フェーズ(高額・中古品出品フロー)実装完了**:
  - `shop-core`: `Category`に`Car`/`Motorcycle`/`Airplane`を追加
    (`is_high_value_vehicle()`ヘルパー付き)。`VehicleDetails`型を
    新設(`model`/`year`/`mileage_km`/`flight_hours`/
    `registration_number`/`has_maintenance_record`/
    `has_accident_history`、全フィールド`Option`)。`Product`に
    `vehicle_details: Option<VehicleDetails>`と
    `used_condition_note: Option<String>`を追加。いずれも`#[serde(default)]`
    かつ`Option`のため、既存カテゴリ(家具等)・既存テストとの後方互換
    を壊していない。
  - `shop-api`: `POST /v1/products`に2つのバリデーション関数を追加。
    - `validate_vehicle_details`: `car`/`motorcycle`/`airplane`は
      `vehicle_details`必須。年式(`year`)・登録番号/機体記号
      (`registration_number`)は共通必須、自動車/バイクは
      `mileage_km`、航空機は`flight_hours`がそれぞれ追加で必須
      (航空機に「走行距離」の概念が無いため項目を分けた)。欠けている
      フィールド名を含む400エラーを返す。
    - `validate_used_condition`: `condition: "used"`の商品は
      `used_condition_note`必須(カテゴリを問わない共通ルール、
      `"new"`では不要でエラーにもしない)。
    - `GET /v1/products/search`に`year_min`/`year_max`/`mileage_max`を
      追加(`parse_u16_param`/`parse_u32_param`ヘルパー新設)。
      `filter_products`にこれら3条件を追加(`vehicle_details`を持たない
      商品はこれらの条件が指定されている場合マッチしないようにし、
      家具等に年式・走行距離という無関係な概念を誤って適用しないよう
      にした)。
  - **テスト**: `shop-core`は既存5件のまま(`cart.rs`のテスト用
    `sample_product`ヘルパーに新フィールドの`None`初期化を追加した
    のみ)。`shop-api`に第2フェーズ用テストを9件追加
    (`vehicle_details`欠如時の400・自動車の`mileage_km`欠如時の
    フィールド名付き400・航空機は`mileage_km`ではなく`flight_hours`
    必須・自動車の完全な出品成功・中古品で`used_condition_note`欠如時の
    400・新品では不要なことの確認・年式範囲/走行距離上限フィルタの
    絞り込み・不正な`year_min`での400)。既存の
    `create_product_with_unknown_category_returns_400_with_valid_list`
    テストは`airplane`が第2フェーズで実在カテゴリになったため、
    無効カテゴリの例を`spaceship`に差し替えた。既存の
    `search_filters_by_keyword_category_price_and_condition`テストは
    `condition: "used"`の商品作成時に`used_condition_note`が必須に
    なったため、テストヘルパー`register_product`を
    `condition == "used"`の場合に自動でメモを付与するよう修正した。
    合計`shop-core` 5件+`shop-api` 24件、全29件成功
    (`cargo test --release`)。
  - **検証結果**: `cargo build --release`成功。実サーバーを起動し、
    PowerShellの`Invoke-RestMethod`で(1)`vehicle_details`無しの
    自動車出品が400(フィールド名`vehicle_details`)、(2)年式・走行距離・
    登録番号を含む中古車の出品成功、(3)`year_min=2015&mileage_max=50000`
    での検索が該当1件のみを返す、(4)`used_condition_note`無しの中古
    家具出品が400、(5)新品家具の出品・検索は従来通り成功、を
    それぞれ実HTTPリクエストで確認した。
  - **スコープ外(意図的に未実装、正直な開示)**: 年式・走行距離・
    登録番号等は自己申告データの保持のみで、車検証・登録証・機体記号
    の公的機関照合や整備記録・事故歴の裏取りは一切行っていない。
    注文確定・決済処理(Stripe連携含む)、保険代理店API連携、国際発送・
    貿易・宅配便API連携、認証・認可、実DB永続化には引き続き一切
    手を付けていない。
  - 次にすべきこと: 第3フェーズ(保険代理店API連携)に着手する場合、
    高額車両(自動車・バイク・航空機)の出品時点で保険加入状況・
    代理店紹介をどう組み込むか検討が必要。また、決済処理
    (第2フェーズのHANDOFFで触れた注文確定フロー)は依然未着手のため、
    フェーズ3着手前にユーザーとスコープを再確認すること。

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
