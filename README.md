# open-shop

一般個人から貿易商社まで出品・出店可能な総合通販システム(構想段階)。
最終的には、高級家具から格安家具、自動車・バイク・航空機まで新品/中古品の
売買、保険代理店連携、世界中の宅配便と連携した国際発送・貿易対応を目指す。
複数フェーズに分割して段階的に実装する方針([`CLAUDE.md`](CLAUDE.md)の
フェーズ計画を参照)。

[English README](README-English.md)

エコシステム共通の開発ルール(Rust中心、実機検証必須、正直な開示徹底、
新規フォルダ作成前の相談ルール、DB非依存・1バイナリ完結・テンプレート
エンジン不使用等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`が
**正本**であり、詳細をここに複製せずそちらを参照する。

## 第2フェーズ(本リポジトリの現状のスコープ)

第1フェーズ(コアEC基盤)に加え、「高額・中古品出品フロー」の
うち以下を実装済み:

- **`Category`に自動車・バイク・航空機を追加**(`car`/`motorcycle`/
  `airplane`)。既存の家具系カテゴリとの共存(後方互換)を保つ。
- **`VehicleDetails`型**(`Product.vehicle_details: Option<...>`)—
  車種・年式・走行距離(自動車/バイク)・総飛行時間(航空機)・
  登録番号/機体記号・整備記録の有無・事故歴の有無を保持。既存
  カテゴリでは`None`のまま(必須化していない)。
- **出品API(`POST /v1/products`)のバリデーション追加**:
  - `car`/`motorcycle`/`airplane`は`vehicle_details`必須。年式・
    登録番号(または機体記号)は共通必須、自動車/バイクは走行距離
    (`mileage_km`)、航空機は総飛行時間(`flight_hours`)がそれぞれ
    必須(欠けていると具体的なフィールド名を含む400エラー)。
  - `condition: "used"`の商品は`used_condition_note`(使用感・状態の
    自由記述)必須。`"new"`では不要(カテゴリを問わない共通ルール)。
- **検索API(`GET /v1/products/search`)の拡張**: 既存の
  `q`/`category`/`price_min`/`price_max`/`condition`に加え、
  `year_min`/`year_max`/`mileage_max`で年式範囲・走行距離上限を
  絞り込み可能(`vehicle_details`を持たない商品にはこれらの条件は
  マッチしない)。

## 第1フェーズ

「コアEC基盤(出品・出店・商品検索・カート)」のみを実装している。

- **出品者/出店者モデル**: 個人出品者(`individual`、表示名のみ)と
  法人・個人事業主の出店者(`business`、屋号を保持)を区別する
  `Seller`型。
- **商品モデル**: カテゴリ(家具の一般的な小分類: ソファ・テーブル・
  椅子・ベッド・棚・照明・その他)、価格(円)、在庫数、新品/中古、
  出品者ID、画像URL(実ファイルは扱わずURL文字列のみ)を持つ`Product`型。
- **出品API**: `POST /v1/products` — 商品登録。
- **商品検索API**: `GET /v1/products/search` — キーワード・カテゴリ・
  価格帯・新品/中古でのフィルタ検索。
- **カートAPI**: カートへの追加・削除・一覧・合計金額計算。
  **注文確定(決済)は未実装**(次フェーズ)。

## 構成

```
open-shop/
├── Cargo.toml (workspace)
├── crates/
│   ├── shop-core/   # データモデル(Seller/Product/Cart)、決済処理を含まない
│   └── shop-api/    # tokio/hyper直接実装のHTTP API
```

`open-fudousan/crates/fudousan-api/src/main.rs`の設計パターン(hyperの
`Incoming`に依存しない同期の純粋関数`route()`へルーティングを分離し、
実サーバー起動なしで単体テスト可能にする、丁寧なエラーメッセージを
返す)を踏襲している。

## エンドポイント一覧

| メソッド | パス | 説明 |
|---|---|---|
| POST | `/v1/sellers` | 出品者/出店者登録 |
| POST | `/v1/products` | 出品(商品登録) |
| GET | `/v1/products/search?q=&category=&price_min=&price_max=&condition=&year_min=&year_max=&mileage_max=` | 商品検索(年式範囲・走行距離上限は自動車/バイク/航空機のみ対象) |
| GET | `/v1/products/:id` | 商品詳細 |
| POST | `/v1/carts` | カート新規作成 |
| GET | `/v1/carts/:id` | カート一覧+合計金額 |
| POST | `/v1/carts/:id/items` | カートに商品を追加 |
| DELETE | `/v1/carts/:id/items/:product_id` | カートから商品を削除 |
| GET | `/healthz` | ヘルスチェック |

## 使い方

```powershell
cargo build --release
target\release\shop-api.exe   # 既定で http://127.0.0.1:8081 (環境変数 SHOP_PORT で変更可)
```

出品者登録:

```powershell
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8081/v1/sellers `
  -ContentType "application/json; charset=utf-8" `
  -Body '{"kind":"business","shop_name":"サンプル家具店","contact_email":"shop@example.com"}'
```

商品登録・検索・カート操作の具体例は[`CLAUDE.md`](CLAUDE.md)のHANDOFF、
または`crates/shop-api/src/main.rs`のテストコードを参照。

## 今後のフェーズ計画

1. **コアEC基盤(実装済み)** — 出品・出店・商品検索・カート。
2. **高額・中古品出品フロー(本フェーズ、実装済み)** — 自動車・バイク・
   航空機の出品項目(年式・走行距離/飛行時間・登録番号/機体記号・
   整備記録・事故歴)と、中古品共通の使用感必須バリデーション。
   **注文確定・決済処理(Stripe連携等)は本フェーズでも未実装**
   (次フェーズ以降)。
3. **保険代理店API連携**。
4. **国際発送・貿易対応**(世界中の宅配便との連携)。

## 正直な開示(未実装・スコープ外)

- 注文確定・決済処理(Stripe連携含む)は一切実装していない。カートの
  合計金額計算までで止めている。
- 高額・中古品の出品項目(年式・走行距離・登録番号等)は自己申告の
  データを保持するのみで、車検証・登録証・機体記号の公的機関との
  照合や、整備記録・事故歴の裏取り(真正性検証)は一切行っていない。
- 保険代理店API連携、国際発送・貿易・宅配便API連携は未実装
  (次フェーズ以降)。注文確定・決済処理(Stripe連携含む)も同様に
  未実装。
- 認証・認可、ページネーション、実DB永続化(現状はプロセス内
  インメモリのみ、再起動でデータは失われる)は未実装。
- 画像は実ファイルを扱わず、URL文字列を保持するのみ(アップロード
  機能なし)。
