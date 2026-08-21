//! カートモデル。
//!
//! **第1フェーズのスコープ**: カートへの追加・削除・一覧・合計金額計算
//! までとし、注文確定・決済(Stripe連携含む)は次フェーズ以降で実装する。
//! このファイルにはその決済処理そのものは一切含まれていない。

use serde::{Deserialize, Serialize};

use crate::product::Product;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cart {
    pub id: String,
    pub items: Vec<CartItem>,
}

impl Cart {
    pub fn new(id: impl Into<String>) -> Self {
        Cart { id: id.into(), items: Vec::new() }
    }

    /// 商品を追加する。既にカートに入っている商品なら数量を加算する。
    pub fn add_item(&mut self, product_id: &str, quantity: u32) {
        if let Some(item) = self.items.iter_mut().find(|i| i.product_id == product_id) {
            item.quantity += quantity;
        } else {
            self.items.push(CartItem { product_id: product_id.to_string(), quantity });
        }
    }

    /// 商品をカートから完全に取り除く。存在しなければ何もしない
    /// (呼び出し側で見つかったかどうかを判定したい場合は戻り値を見る)。
    pub fn remove_item(&mut self, product_id: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.product_id != product_id);
        self.items.len() != before
    }

    /// カート内の商品情報から合計金額(円)を計算する。カートに入っている
    /// が既に商品マスタから削除された(見つからない)商品は無視する
    /// (在庫切れ・出品取り下げ後の整合性は第1フェーズでは厳密に扱わない)。
    pub fn total_jpy(&self, products: &[Product]) -> u64 {
        self.items
            .iter()
            .filter_map(|item| {
                products
                    .iter()
                    .find(|p| p.id == item.product_id)
                    .map(|p| p.price_jpy.saturating_mul(item.quantity as u64))
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::product::{Category, Condition};

    fn sample_product(id: &str, price: u64) -> Product {
        Product {
            id: id.to_string(),
            seller_id: "seller-1".to_string(),
            name: "テスト商品".to_string(),
            description: "".to_string(),
            category: Category::Chair,
            price_jpy: price,
            stock_quantity: 10,
            condition: Condition::New,
            image_urls: vec![],
        }
    }

    #[test]
    fn add_item_accumulates_quantity_for_same_product() {
        let mut cart = Cart::new("c1");
        cart.add_item("p1", 2);
        cart.add_item("p1", 3);
        assert_eq!(cart.items.len(), 1);
        assert_eq!(cart.items[0].quantity, 5);
    }

    #[test]
    fn remove_item_removes_only_matching_product() {
        let mut cart = Cart::new("c1");
        cart.add_item("p1", 1);
        cart.add_item("p2", 1);
        assert!(cart.remove_item("p1"));
        assert_eq!(cart.items.len(), 1);
        assert_eq!(cart.items[0].product_id, "p2");
    }

    #[test]
    fn remove_item_returns_false_when_not_found() {
        let mut cart = Cart::new("c1");
        assert!(!cart.remove_item("does-not-exist"));
    }

    #[test]
    fn total_jpy_sums_price_times_quantity() {
        let products = vec![sample_product("p1", 1000), sample_product("p2", 500)];
        let mut cart = Cart::new("c1");
        cart.add_item("p1", 2);
        cart.add_item("p2", 3);
        assert_eq!(cart.total_jpy(&products), 1000 * 2 + 500 * 3);
    }

    #[test]
    fn total_jpy_ignores_products_no_longer_in_catalog() {
        let products = vec![sample_product("p1", 1000)];
        let mut cart = Cart::new("c1");
        cart.add_item("p1", 1);
        cart.add_item("removed-product", 5);
        assert_eq!(cart.total_jpy(&products), 1000);
    }
}
