//! open-shop コアデータモデル(第1フェーズ: コアEC基盤のみ)。
//!
//! **正直な開示**: 決済処理(Stripe連携含む)・保険代理店連携・
//! 国際発送/貿易対応・高額中古品特有の項目(車検証・走行距離等)は
//! いずれも未実装。詳細は`README.md`/`CLAUDE.md`のフェーズ計画を参照。

pub mod cart;
pub mod product;
pub mod seller;

pub use cart::{Cart, CartItem};
pub use product::{Category, Condition, Product};
pub use seller::{Seller, SellerKind};
