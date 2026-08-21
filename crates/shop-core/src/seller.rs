//! 出品者/出店者モデル。
//!
//! 個人(フリマ的な単発出品者)と法人・個人事業主(継続的に出店する
//! 「商店」)を区別する。第1フェーズでは本人確認・審査フロー等は
//! 実装せず、単純なデータモデルのみ。

use serde::{Deserialize, Serialize};

/// 出品者の種別。将来、法人出店者には出店契約・手数料率等の追加
/// フィールドが必要になる見込みだが、第1フェーズでは種別と識別情報の
/// みを保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SellerKind {
    /// 個人出品者。氏名(または表示名)のみ。
    Individual { display_name: String },
    /// 法人・個人事業主の出店者(商店)。屋号を保持する。
    Business { shop_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seller {
    pub id: String,
    #[serde(flatten)]
    pub kind: SellerKind,
    pub contact_email: String,
    pub contact_phone: Option<String>,
}

impl Seller {
    /// 一覧・検索結果に表示する名前(個人なら表示名、法人なら屋号)。
    pub fn display_name(&self) -> &str {
        match &self.kind {
            SellerKind::Individual { display_name } => display_name,
            SellerKind::Business { shop_name } => shop_name,
        }
    }
}
