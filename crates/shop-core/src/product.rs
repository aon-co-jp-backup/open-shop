//! 商品モデル。
//!
//! **第1フェーズのスコープ**: 家具程度の一般的なカテゴリのみを具体化する。
//! 自動車・バイク・航空機や、それら高額・中古品特有の項目(車検証・
//! 走行距離・保険代理店連携等)は次フェーズ以降の対象であり、本モデルには
//! 含めない(`README.md`/`CLAUDE.md`のフェーズ計画を参照)。

use serde::{Deserialize, Serialize};

/// 商品カテゴリ。将来、自動車・バイク・航空機等の大分類を追加できるよう
/// ツリー構造を見据えた設計だが、第1フェーズでは家具の小分類のみを
/// 具体的な列挙子として持つ(汎用ツリー構造そのものの実装は次フェーズ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Sofa,
    Table,
    Chair,
    Bed,
    Shelf,
    Lighting,
    Other,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Sofa => "sofa",
            Category::Table => "table",
            Category::Chair => "chair",
            Category::Bed => "bed",
            Category::Shelf => "shelf",
            Category::Lighting => "lighting",
            Category::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Category> {
        match s {
            "sofa" => Some(Category::Sofa),
            "table" => Some(Category::Table),
            "chair" => Some(Category::Chair),
            "bed" => Some(Category::Bed),
            "shelf" => Some(Category::Shelf),
            "lighting" => Some(Category::Lighting),
            "other" => Some(Category::Other),
            _ => None,
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Sofa,
            Category::Table,
            Category::Chair,
            Category::Bed,
            Category::Shelf,
            Category::Lighting,
            Category::Other,
        ]
    }
}

/// 新品/中古の別。第1フェーズでは真偽値程度の単純な区別のみ
/// (中古品の状態ランク・車検証等の詳細項目は次フェーズ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    New,
    Used,
}

impl Condition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Condition::New => "new",
            Condition::Used => "used",
        }
    }

    pub fn from_str(s: &str) -> Option<Condition> {
        match s {
            "new" => Some(Condition::New),
            "used" => Some(Condition::Used),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub seller_id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    /// 価格(円、税込表示を想定するが第1フェーズでは税処理そのものは
    /// 実装しない)。
    pub price_jpy: u64,
    pub stock_quantity: u32,
    pub condition: Condition,
    /// 実ファイルは扱わず、URL文字列のみを保持する(第1フェーズの方針)。
    pub image_urls: Vec<String>,
}
