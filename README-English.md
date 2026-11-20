# open-shop

A general-purpose e-commerce marketplace intended to let anyone — from
individual sellers to trading companies — list and sell items, from
high-end to budget furniture, and eventually new/used cars, motorcycles,
and even aircraft, with insurance-agency integration and international
shipping/customs support. This is a long-term vision being built in
stages; see [`CLAUDE.md`](CLAUDE.md) for the phase plan.

[日本語README](README.md)

Ecosystem-wide development rules (Rust-first, mandatory real-environment
verification, honest disclosure, ask-before-creating-new-folders,
DB-independent single-binary builds, no template engines, etc.) live in
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)'s `CLAUDE.md`,
which is the canonical source — not duplicated here.

## Phase 1 (current scope of this repository)

Only the "core e-commerce foundation (listing, storefronts, product
search, cart)" is implemented.

- **Seller model**: distinguishes individual sellers (`individual`,
  display name only) from business/storefront sellers (`business`, shop
  name).
- **Product model**: category (a general furniture taxonomy: sofa,
  table, chair, bed, shelf, lighting, other), price (JPY), stock
  quantity, new/used flag, seller ID, image URLs (URL strings only, no
  file uploads).
- **Listing API**: `POST /v1/products`.
- **Product search API**: `GET /v1/products/search` — filter by
  keyword, category, price range, and condition.
- **Cart API**: add/remove/list items and compute totals. **Checkout
  and payment are not implemented** (next phase).

## Layout

```
open-shop/
├── Cargo.toml (workspace)
├── crates/
│   ├── shop-core/   # data models (Seller/Product/Cart), no payment logic
│   └── shop-api/    # HTTP API, direct tokio/hyper implementation
```

Follows the design pattern of
`open-fudousan/crates/fudousan-api/src/main.rs`: routing is split into a
synchronous pure function `route()` independent of hyper's `Incoming`
body, so it's unit-testable without starting a real server, and error
responses include actionable messages.

## Endpoints

| Method | Path | Description |
|---|---|---|
| POST | `/v1/sellers` | Register a seller/storefront |
| POST | `/v1/products` | List a product |
| GET | `/v1/products/search?q=&category=&price_min=&price_max=&condition=` | Product search |
| GET | `/v1/products/:id` | Product detail |
| POST | `/v1/carts` | Create a cart |
| GET | `/v1/carts/:id` | List cart items + total |
| POST | `/v1/carts/:id/items` | Add item to cart |
| DELETE | `/v1/carts/:id/items/:product_id` | Remove item from cart |
| GET | `/healthz` | Health check |

## Usage

```powershell
cargo build --release
target\release\shop-api.exe   # defaults to http://127.0.0.1:8081 (override with SHOP_PORT)
```

## Future phases

1. **Core e-commerce foundation (this phase, implemented)** — listing,
   storefronts, search, cart.
2. **High-value / used-goods listing flow** — car/motorcycle/aircraft
   listing fields (registration docs, mileage, etc.), checkout and
   payment processing (e.g. Stripe integration).
3. **Insurance agency API integration**.
4. **International shipping and trade support** (integration with
   worldwide courier services).

## Honest disclosure (not implemented / out of scope)

- Checkout and payment processing (including Stripe) are not
  implemented at all; the cart stops at total calculation.
- High-value/used-goods-specific listing fields, insurance agency
  integration, and international shipping/trade/courier API
  integration are not implemented (future phases).
- Authentication/authorization, pagination, and real database
  persistence (currently in-process in-memory only; data is lost on
  restart) are not implemented.
- Images are URL strings only; no file upload capability.
