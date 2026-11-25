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

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**

## Versionless API の方針 / Versionless API policy (2026-11-25)
**日本語**: このリポジトリのAPIは **Versionless**(URL・パッケージ名に `v1` 等を付けず、互換性を壊さず足すだけで進化させる)。欄は足すだけ・足す欄は省略可能・知らない欄は無視・名前変更は新名を足して旧を非推奨に(期限は既定で6か月以上、使用0を確認してから削除)・enumは未知の値を許容・安全上の緊急修正は例外。既存の `.v1` は呼び出し側を壊さないよう当面残し、版なしの別名を追加して段階的に畳む。詳細・出典は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md) 末尾。適用はこれから順に行う。
**English**: This repository's APIs are **Versionless**: no `v1` in URLs or package names; evolve by adding only, without breaking compatibility. Only add fields, new fields are optional, ignore unknown fields, rename by adding the new name and deprecating the old (default deadline 6 months or more, remove only after confirming zero use), tolerate unknown enum values, emergency security fixes are the exception. Existing `.v1` names stay for now so callers do not break; a versionless alias is added and `.v1` is retired in stages. Details and sources: the end of [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md). Rollout is step by step.
