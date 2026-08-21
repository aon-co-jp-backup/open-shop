//! 商品モデル。
//!
//! **第1フェーズのスコープ**: 家具程度の一般的なカテゴリのみを具体化した。
//! **第2フェーズ(本フェーズ)**: 自動車・バイク・航空機の高額・中古品
//! カテゴリを追加し、それら特有の項目(年式・走行距離・登録番号/機体
//! 記号・整備記録・事故歴等)を`VehicleDetails`として保持できるように
//! した。保険代理店連携・国際発送/貿易対応は次フェーズ以降のまま
//! 含めない(`README.md`/`CLAUDE.md`のフェーズ計画を参照)。

use serde::{Deserialize, Serialize};

/// 商品カテゴリ。将来、自動車・バイク・航空機等の大分類を追加できるよう
/// ツリー構造を見据えた設計だが、具体的な列挙子として持つ(汎用ツリー
/// 構造そのものの実装は次フェーズ以降)。
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
    /// 高額・中古品カテゴリ(第2フェーズで追加)。出品時に
    /// `VehicleDetails`の必須項目が検証される(`shop-api`側)。
    Car,
    Motorcycle,
    Airplane,
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
            Category::Car => "car",
            Category::Motorcycle => "motorcycle",
            Category::Airplane => "airplane",
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
            "car" => Some(Category::Car),
            "motorcycle" => Some(Category::Motorcycle),
            "airplane" => Some(Category::Airplane),
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
            Category::Car,
            Category::Motorcycle,
            Category::Airplane,
        ]
    }

    /// 高額・中古品カテゴリ(自動車・バイク・航空機)かどうか。
    /// `shop-api`の出品バリデーションで`VehicleDetails`必須項目の
    /// チェックに使う。
    pub fn is_high_value_vehicle(&self) -> bool {
        matches!(self, Category::Car | Category::Motorcycle | Category::Airplane)
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

/// 自動車・バイク・航空機の高額・中古品特有の出品項目(第2フェーズ)。
///
/// **正直な開示**: ここに保持するのは自己申告データであり、車検証・
/// 登録証・機体記号の公的機関との照合、整備記録・事故歴の裏取りは
/// 一切行わない(そうした真正性検証は次フェーズ以降のスコープ外事項)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VehicleDetails {
    /// 車種・型式(例: 「プリウス」「CB400SF」「セスナ172」)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 年式(西暦)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// 走行距離(km)。自動車・バイクで使用。航空機では飛行時間の方が
    /// 一般的だが、第2フェーズでは項目を増やさずこのフィールドを
    /// 流用せず、航空機は`flight_hours`を使う。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mileage_km: Option<u32>,
    /// 総飛行時間(時間)。航空機で使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flight_hours: Option<u32>,
    /// 登録番号(自動車のナンバープレート・バイクの車体番号)または
    /// 航空機の機体記号(レジ番号)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,
    /// 整備記録簿の有無。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_maintenance_record: Option<bool>,
    /// 事故歴の有無。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_accident_history: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub seller_id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    /// 価格(円、税込表示を想定するが税処理そのものは実装しない)。
    pub price_jpy: u64,
    pub stock_quantity: u32,
    pub condition: Condition,
    /// 実ファイルは扱わず、URL文字列のみを保持する(第1フェーズの方針)。
    pub image_urls: Vec<String>,
    /// 自動車・バイク・航空機カテゴリの高額・中古品特有の項目(第2フェーズ)。
    /// 既存カテゴリ(家具等)では`None`のまま(後方互換のため必須化しない)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicle_details: Option<VehicleDetails>,
    /// 中古品(`Condition::Used`)の使用感・状態を説明する自由記述。
    /// `Condition::New`では不要(未設定でよい)。既存カテゴリ・既存
    /// テストとの後方互換のため`Option`かつ`#[serde(default)]`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub used_condition_note: Option<String>,
}
