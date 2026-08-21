//! open-shop API サーバー(第1フェーズ: コアEC基盤 = 出品・出店者登録・
//! 商品検索・カートのみ)。
//!
//! tokio/hyperを直接用いた自前実装(重量級フレームワーク・ORM不使用の
//! エコシステム方針に準拠、`open-raid-z/CLAUDE.md`参照)。
//! ルーティングは`open-fudousan/crates/fudousan-api/src/main.rs`と同様、
//! hyperの`Incoming`ボディに依存しない同期の純粋関数へ分離し、単体テスト
//! 可能にしている。
//!
//! # エンドポイント
//! - `POST /v1/sellers` — 出品者/出店者登録。
//! - `POST /v1/products` — 出品(商品登録)。
//! - `GET /v1/products/search?q=&category=&price_min=&price_max=&condition=`
//!   — キーワード・カテゴリ・価格帯・新品/中古でのフィルタ検索。
//! - `GET /v1/products/:id` — 商品詳細。
//! - `POST /v1/carts` — カート新規作成。
//! - `GET /v1/carts/:id` — カート一覧+合計金額。
//! - `POST /v1/carts/:id/items` — カートに商品を追加。
//! - `DELETE /v1/carts/:id/items/:product_id` — カートから商品を削除。
//! - `GET /healthz`
//!
//! **正直な開示**: 認証・ページネーション・実DB永続化(プロセス内
//! インメモリのみ、再起動でデータは失われる)は未実装。**注文確定・
//! 決済(Stripe連携含む)は本APIに一切含まれていない**(次フェーズ、
//! `README.md`/`CLAUDE.md`のフェーズ計画を参照)。

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use serde::Deserialize;
use shop_core::{Cart, Category, Condition, Product, Seller, SellerKind};
use tokio::net::TcpListener;

type BoxBody = Full<Bytes>;

struct AppState {
    sellers: Mutex<HashMap<String, Seller>>,
    products: Mutex<HashMap<String, Product>>,
    carts: Mutex<HashMap<String, Cart>>,
    next_id: Mutex<u64>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            sellers: Mutex::new(HashMap::new()),
            products: Mutex::new(HashMap::new()),
            carts: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    fn fresh_id(&self, prefix: &str) -> String {
        let mut n = self.next_id.lock().unwrap();
        let id = format!("{prefix}-{n}");
        *n += 1;
        id
    }
}

fn json_response(status: StatusCode, value: &impl serde::Serialize) -> Response<BoxBody> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn bad_request(field: &str, message: &str) -> Response<BoxBody> {
    json_response(
        StatusCode::BAD_REQUEST,
        &serde_json::json!({ "error": "bad_request", "field": field, "message": message }),
    )
}

fn not_found(kind: &str, id: &str) -> Response<BoxBody> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({
            "error": "not_found",
            "message": format!("no {kind} with id '{id}'"),
        }),
    )
}

/// クエリ文字列 `a=1&b=2` を単純パースする(`open-fudousan`と同一方針、
/// 依存クレート追加を避けるための簡易実装)。
fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            Some((simple_url_decode(k), simple_url_decode(v)))
        })
        .collect()
}

fn simple_url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SellerRequest {
    Individual { display_name: String, contact_email: String, contact_phone: Option<String> },
    Business { shop_name: String, contact_email: String, contact_phone: Option<String> },
}

#[derive(Deserialize)]
struct ProductRequest {
    seller_id: String,
    name: String,
    #[serde(default)]
    description: String,
    category: String,
    price_jpy: u64,
    #[serde(default)]
    stock_quantity: u32,
    condition: String,
    #[serde(default)]
    image_urls: Vec<String>,
}

#[derive(Deserialize)]
struct CartItemRequest {
    product_id: String,
    #[serde(default = "default_quantity")]
    quantity: u32,
}

fn default_quantity() -> u32 {
    1
}

/// メソッド・パス・クエリ文字列・(POST/DELETE時の)ボディからルーティング
/// を行い、レスポンスを組み立てる純粋関数。hyperの`Incoming`に依存しない
/// ため単体テストで実サーバーを起動せずに検証できる。
fn route(method: &Method, path: &str, query: &str, body: &[u8], state: &AppState) -> Response<BoxBody> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    match (method, segments.as_slice()) {
        (&Method::GET, ["healthz"]) => json_response(StatusCode::OK, &serde_json::json!({ "status": "ok" })),

        (&Method::POST, ["v1", "sellers"]) => handle_create_seller(body, state),

        (&Method::POST, ["v1", "products"]) => handle_create_product(body, state),

        (&Method::GET, ["v1", "products", "search"]) => handle_search_products(query, state),

        (&Method::GET, ["v1", "products", id]) => {
            let products = state.products.lock().unwrap();
            match products.get(*id) {
                Some(p) => json_response(StatusCode::OK, p),
                None => not_found("product", id),
            }
        }

        (&Method::POST, ["v1", "carts"]) => {
            let id = state.fresh_id("cart");
            let cart = Cart::new(id.clone());
            state.carts.lock().unwrap().insert(id, cart.clone());
            json_response(StatusCode::OK, &cart)
        }

        (&Method::GET, ["v1", "carts", id]) => {
            let carts = state.carts.lock().unwrap();
            match carts.get(*id) {
                Some(cart) => {
                    let products = state.products.lock().unwrap();
                    let all_products: Vec<Product> = products.values().cloned().collect();
                    let total = cart.total_jpy(&all_products);
                    json_response(StatusCode::OK, &serde_json::json!({
                        "id": cart.id,
                        "items": cart.items,
                        "total_jpy": total,
                    }))
                }
                None => not_found("cart", id),
            }
        }

        (&Method::POST, ["v1", "carts", cart_id, "items"]) => handle_add_cart_item(cart_id, body, state),

        (&Method::DELETE, ["v1", "carts", cart_id, "items", product_id]) => {
            let mut carts = state.carts.lock().unwrap();
            match carts.get_mut(*cart_id) {
                Some(cart) => {
                    let removed = cart.remove_item(product_id);
                    if removed {
                        json_response(StatusCode::OK, &serde_json::json!({ "removed": true }))
                    } else {
                        not_found("cart item (product)", product_id)
                    }
                }
                None => not_found("cart", cart_id),
            }
        }

        (m, _) if *m != Method::GET && *m != Method::POST && *m != Method::DELETE => json_response(
            StatusCode::METHOD_NOT_ALLOWED,
            &serde_json::json!({
                "error": "method_not_allowed",
                "message": format!("unsupported method {m}"),
            }),
        ),

        _ => json_response(
            StatusCode::NOT_FOUND,
            &serde_json::json!({
                "error": "not_found",
                "message": "unknown route; see POST /v1/sellers, POST /v1/products, GET /v1/products/search, GET /v1/products/:id, POST /v1/carts, GET /v1/carts/:id, POST /v1/carts/:id/items, DELETE /v1/carts/:id/items/:product_id, GET /healthz",
            }),
        ),
    }
}

fn handle_create_seller(body: &[u8], state: &AppState) -> Response<BoxBody> {
    let req: SellerRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => return bad_request("body", &format!("invalid seller JSON: {e}")),
    };

    let id = state.fresh_id("seller");
    let seller = match req {
        SellerRequest::Individual { display_name, contact_email, contact_phone } => Seller {
            id: id.clone(),
            kind: SellerKind::Individual { display_name },
            contact_email,
            contact_phone,
        },
        SellerRequest::Business { shop_name, contact_email, contact_phone } => Seller {
            id: id.clone(),
            kind: SellerKind::Business { shop_name },
            contact_email,
            contact_phone,
        },
    };

    state.sellers.lock().unwrap().insert(id, seller.clone());
    json_response(StatusCode::OK, &seller)
}

fn handle_create_product(body: &[u8], state: &AppState) -> Response<BoxBody> {
    let req: ProductRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => return bad_request("body", &format!("invalid product JSON: {e}")),
    };

    if !state.sellers.lock().unwrap().contains_key(&req.seller_id) {
        return bad_request(
            "seller_id",
            &format!("seller_id '{}' does not exist; register the seller via POST /v1/sellers first", req.seller_id),
        );
    }

    let category = match Category::from_str(&req.category) {
        Some(c) => c,
        None => {
            let valid: Vec<&str> = Category::all().iter().map(|c| c.as_str()).collect();
            return bad_request("category", &format!("unknown category '{}'; valid values: {}", req.category, valid.join(", ")));
        }
    };

    let condition = match Condition::from_str(&req.condition) {
        Some(c) => c,
        None => {
            return bad_request("condition", &format!("unknown condition '{}'; valid values: new, used", req.condition));
        }
    };

    let id = state.fresh_id("product");
    let product = Product {
        id: id.clone(),
        seller_id: req.seller_id,
        name: req.name,
        description: req.description,
        category,
        price_jpy: req.price_jpy,
        stock_quantity: req.stock_quantity,
        condition,
        image_urls: req.image_urls,
    };

    state.products.lock().unwrap().insert(id, product.clone());
    json_response(StatusCode::OK, &product)
}

/// 検索クエリのフィルタ条件をクエリ文字列から取り出したうえで
/// 商品一覧へ適用する。フィルタ条件の適用自体は`filter_products`(下記)
/// に切り出してあり、こちらは単体テストしやすい。
fn handle_search_products(query: &str, state: &AppState) -> Response<BoxBody> {
    let params = parse_query(query);
    let get = |key: &str| params.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());

    let keyword = get("q").filter(|s| !s.is_empty());

    let category = match get("category").filter(|s| !s.is_empty()) {
        None => None,
        Some(raw) => match Category::from_str(&raw) {
            Some(c) => Some(c),
            None => {
                let valid: Vec<&str> = Category::all().iter().map(|c| c.as_str()).collect();
                return bad_request("category", &format!("unknown category '{raw}'; valid values: {}", valid.join(", ")));
            }
        },
    };

    let condition = match get("condition").filter(|s| !s.is_empty()) {
        None => None,
        Some(raw) => match Condition::from_str(&raw) {
            Some(c) => Some(c),
            None => return bad_request("condition", &format!("unknown condition '{raw}'; valid values: new, used")),
        },
    };

    let price_min = match parse_price_param(get("price_min"), "price_min") {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let price_max = match parse_price_param(get("price_max"), "price_max") {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let products = state.products.lock().unwrap();
    let all: Vec<Product> = products.values().cloned().collect();
    let results = filter_products(&all, keyword.as_deref(), category, price_min, price_max, condition);

    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "count": results.len(),
            "results": results,
        }),
    )
}

fn parse_price_param(raw: Option<String>, field: &str) -> Result<Option<u64>, Response<BoxBody>> {
    match raw {
        None => Ok(None),
        Some(ref s) if s.is_empty() => Ok(None),
        Some(ref s) => s.parse::<u64>().map(Some).map_err(|_| {
            bad_request(field, &format!("{field} must be a non-negative integer (yen), e.g. {field}=10000"))
        }),
    }
}

/// キーワード(商品名・説明文の部分一致)・カテゴリ・価格帯・新品/中古で
/// 商品一覧を絞り込む純粋関数。HTTP/クエリパースから独立しているため
/// フィルタロジックだけを単体テストできる。
fn filter_products<'a>(
    products: &'a [Product],
    keyword: Option<&str>,
    category: Option<Category>,
    price_min: Option<u64>,
    price_max: Option<u64>,
    condition: Option<Condition>,
) -> Vec<&'a Product> {
    products
        .iter()
        .filter(|p| {
            keyword
                .map(|kw| p.name.contains(kw) || p.description.contains(kw))
                .unwrap_or(true)
        })
        .filter(|p| category.map(|c| p.category == c).unwrap_or(true))
        .filter(|p| price_min.map(|min| p.price_jpy >= min).unwrap_or(true))
        .filter(|p| price_max.map(|max| p.price_jpy <= max).unwrap_or(true))
        .filter(|p| condition.map(|c| p.condition == c).unwrap_or(true))
        .collect()
}

fn handle_add_cart_item(cart_id: &str, body: &[u8], state: &AppState) -> Response<BoxBody> {
    let req: CartItemRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => return bad_request("body", &format!("invalid cart item JSON: {e}")),
    };

    if !state.products.lock().unwrap().contains_key(&req.product_id) {
        return bad_request("product_id", &format!("product_id '{}' does not exist", req.product_id));
    }

    if req.quantity == 0 {
        return bad_request("quantity", "quantity must be at least 1");
    }

    let mut carts = state.carts.lock().unwrap();
    match carts.get_mut(cart_id) {
        Some(cart) => {
            cart.add_item(&req.product_id, req.quantity);
            json_response(StatusCode::OK, cart)
        }
        None => not_found("cart", cart_id),
    }
}

async fn handle(state: Arc<AppState>, req: Request<Incoming>) -> Result<Response<BoxBody>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let body = req.into_body().collect().await.map(|c| c.to_bytes()).unwrap_or_default();

    Ok(route(&method, &path, &query, &body, &state))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState::new());

    let port: u16 = std::env::var("SHOP_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8081);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("open-shop API listening on http://{addr} (in-memory only, no payment processing)");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = state.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| handle(state.clone(), req));
            if let Err(err) = ConnBuilder::new(TokioExecutor::new()).serve_connection(io, service).await {
                eprintln!("connection error: {err}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> AppState {
        AppState::new()
    }

    async fn body_json(resp: Response<BoxBody>) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.expect("infallible").to_bytes();
        serde_json::from_slice(&bytes).expect("response body must be JSON")
    }

    fn register_individual_seller(state: &AppState) -> String {
        let resp = route(
            &Method::POST,
            "/v1/sellers",
            "",
            r#"{"kind":"individual","display_name":"山田太郎","contact_email":"yamada@example.com"}"#.as_bytes(),
            state,
        );
        assert_eq!(resp.status(), StatusCode::OK);
        tokio_test_block_on(async { body_json(resp).await["id"].as_str().unwrap().to_string() })
    }

    // hyperのFullボディは即値なので、テスト内では簡易ブロッキング実行で十分。
    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f)
    }

    fn register_product(state: &AppState, seller_id: &str, name: &str, category: &str, price: u64, condition: &str) -> String {
        let body = serde_json::json!({
            "seller_id": seller_id,
            "name": name,
            "description": format!("{name}の説明"),
            "category": category,
            "price_jpy": price,
            "stock_quantity": 5,
            "condition": condition,
            "image_urls": ["https://example.com/img.jpg"],
        });
        let resp = route(&Method::POST, "/v1/products", "", body.to_string().as_bytes(), state);
        assert_eq!(resp.status(), StatusCode::OK);
        tokio_test_block_on(async { body_json(resp).await["id"].as_str().unwrap().to_string() })
    }

    #[test]
    fn parse_query_decodes_percent_and_plus() {
        let params = parse_query("q=%E6%A4%85%E5%AD%90&price_max=100");
        assert_eq!(params[0], ("q".to_string(), "椅子".to_string()));
    }

    #[test]
    fn healthz_returns_ok() {
        let state = empty_state();
        let resp = route(&Method::GET, "/healthz", "", b"", &state);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn create_seller_individual_succeeds() {
        let state = empty_state();
        let id = register_individual_seller(&state);
        assert!(id.starts_with("seller-"));
        assert_eq!(state.sellers.lock().unwrap().len(), 1);
    }

    #[test]
    fn create_seller_business_succeeds() {
        let state = empty_state();
        let resp = route(
            &Method::POST,
            "/v1/sellers",
            "",
            r#"{"kind":"business","shop_name":"株式会社サンプル家具","contact_email":"shop@example.com","contact_phone":"03-0000-0000"}"#.as_bytes(),
            &state,
        );
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn create_seller_with_invalid_json_returns_400() {
        let state = empty_state();
        let resp = route(&Method::POST, "/v1/sellers", "", b"not json", &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn create_product_requires_existing_seller() {
        let state = empty_state();
        let body = serde_json::json!({
            "seller_id": "does-not-exist",
            "name": "テストソファ",
            "category": "sofa",
            "price_jpy": 50000,
            "condition": "new",
        });
        let resp = route(&Method::POST, "/v1/products", "", body.to_string().as_bytes(), &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["field"], "seller_id");
    }

    #[test]
    fn create_product_with_unknown_category_returns_400_with_valid_list() {
        let state = empty_state();
        let seller_id = register_individual_seller(&state);
        let body = serde_json::json!({
            "seller_id": seller_id,
            "name": "テスト",
            "category": "airplane",
            "price_jpy": 1000,
            "condition": "new",
        });
        let resp = route(&Method::POST, "/v1/products", "", body.to_string().as_bytes(), &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = tokio_test_block_on(body_json(resp));
        assert!(json["message"].as_str().unwrap().contains("sofa"));
    }

    #[test]
    fn create_and_fetch_product_roundtrip() {
        let state = empty_state();
        let seller_id = register_individual_seller(&state);
        let product_id = register_product(&state, &seller_id, "北欧風ソファ", "sofa", 45000, "new");

        let resp = route(&Method::GET, &format!("/v1/products/{product_id}"), "", b"", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["name"], "北欧風ソファ");
    }

    #[test]
    fn get_unknown_product_returns_404_with_id_in_message() {
        let state = empty_state();
        let resp = route(&Method::GET, "/v1/products/does-not-exist", "", b"", &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = tokio_test_block_on(body_json(resp));
        assert!(json["message"].as_str().unwrap().contains("does-not-exist"));
    }

    #[test]
    fn search_filters_by_keyword_category_price_and_condition() {
        let state = empty_state();
        let seller_id = register_individual_seller(&state);
        register_product(&state, &seller_id, "北欧風ソファ", "sofa", 45000, "new");
        register_product(&state, &seller_id, "中古の木製テーブル", "table", 8000, "used");
        register_product(&state, &seller_id, "高級ソファ", "sofa", 200000, "new");

        let resp = route(&Method::GET, "/v1/products/search", "category=sofa", b"", &state);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["count"], 2);

        let resp = route(&Method::GET, "/v1/products/search", "category=sofa&price_max=100000", b"", &state);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["count"], 1);

        let resp = route(&Method::GET, "/v1/products/search", "condition=used", b"", &state);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["count"], 1);

        let resp = route(&Method::GET, "/v1/products/search", "q=%E6%9C%A8%E8%A3%BD", b"", &state); // "木製"
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["count"], 1);
    }

    #[test]
    fn search_with_invalid_price_returns_400() {
        let state = empty_state();
        let resp = route(&Method::GET, "/v1/products/search", "price_min=abc", b"", &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["field"], "price_min");
    }

    #[test]
    fn cart_create_add_remove_and_total_flow() {
        let state = empty_state();
        let seller_id = register_individual_seller(&state);
        let product_id = register_product(&state, &seller_id, "椅子", "chair", 3000, "new");

        let resp = route(&Method::POST, "/v1/carts", "", b"", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let cart_id = tokio_test_block_on(body_json(resp))["id"].as_str().unwrap().to_string();

        let add_body = serde_json::json!({ "product_id": product_id, "quantity": 3 });
        let resp = route(&Method::POST, &format!("/v1/carts/{cart_id}/items"), "", add_body.to_string().as_bytes(), &state);
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = route(&Method::GET, &format!("/v1/carts/{cart_id}"), "", b"", &state);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["total_jpy"], 9000);

        let resp = route(&Method::DELETE, &format!("/v1/carts/{cart_id}/items/{product_id}"), "", b"", &state);
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = route(&Method::GET, &format!("/v1/carts/{cart_id}"), "", b"", &state);
        let json = tokio_test_block_on(body_json(resp));
        assert_eq!(json["total_jpy"], 0);
    }

    #[test]
    fn add_item_to_unknown_cart_returns_404() {
        let state = empty_state();
        let seller_id = register_individual_seller(&state);
        let product_id = register_product(&state, &seller_id, "椅子", "chair", 1000, "new");
        let add_body = serde_json::json!({ "product_id": product_id, "quantity": 1 });
        let resp = route(&Method::POST, "/v1/carts/does-not-exist/items", "", add_body.to_string().as_bytes(), &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn add_unknown_product_to_cart_returns_400() {
        let state = empty_state();
        let resp = route(&Method::POST, "/v1/carts", "", b"", &state);
        let cart_id = tokio_test_block_on(body_json(resp))["id"].as_str().unwrap().to_string();

        let add_body = serde_json::json!({ "product_id": "does-not-exist", "quantity": 1 });
        let resp = route(&Method::POST, &format!("/v1/carts/{cart_id}/items"), "", add_body.to_string().as_bytes(), &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unknown_route_returns_404_with_endpoint_hint() {
        let state = empty_state();
        let resp = route(&Method::GET, "/nonexistent", "", b"", &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = tokio_test_block_on(body_json(resp));
        assert!(json["message"].as_str().unwrap().contains("/v1/products/search"));
    }

    #[test]
    fn unsupported_method_returns_405() {
        let state = empty_state();
        let resp = route(&Method::PUT, "/v1/products/search", "", b"", &state);
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
