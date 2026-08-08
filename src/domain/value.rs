//! 数据库值类型系统
//!
//! `DbValue` 是 Gridix 中所有数据库值的统一表示，
//! 取代 `QueryResult` 中的 `Vec<Vec<String>>` + `null_flags` 模式。
//!
//! 每个变体携带足够的类型信息以支持：
//! - 类型感知排序（`Int(100) > Int(20)`，而非字典序）
//! - 类型感知显示（数字右对齐、日期格式化）
//! - 类型感知导出（JSON number vs string）

use std::cmp::Ordering;
use std::sync::Arc;

/// 数据库值的统一表示
#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    /// SQL NULL
    Null,

    /// 布尔值
    Bool(bool),

    /// 有符号整数（涵盖 SQLite INTEGER, PG integer/bigint, MySQL INT/BIGINT）
    Int(i64),

    /// 无符号整数（MySQL UNSIGNED 系列）
    UInt(u64),

    /// 浮点数（涵盖 REAL, DOUBLE, FLOAT）
    Float(f64),

    /// 精确十进制（NUMERIC/DECIMAL，用字符串避免 f64 精度损失）
    Decimal(String),

    /// 文本（VARCHAR, TEXT, CHAR 等）
    Text(String),

    /// 二进制数据（BLOB, BYTEA 等）
    Bytes(Arc<[u8]>),

    /// 日期
    Date(DbDate),

    /// 时间
    Time(DbTime),

    /// 日期时间
    DateTime(DbDateTime),

    /// JSON 值
    Json(serde_json::Value),

    /// UUID
    Uuid(uuid::Uuid),

    /// 数组（PostgreSQL array 等）
    Array(Vec<DbValue>),

    /// 数据库特有类型，保留原始类型名和显示文本
    Other {
        native_type: String,
        display: String,
    },
}

// ── 日期/时间辅助类型 ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanos: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbDateTime {
    pub date: DbDate,
    pub time: DbTime,
}

// ── 类型族（用于粗略分类，不替代完整类型信息）──

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum DbTypeFamily {
    Null,
    Bool,
    Integer,
    Float,
    Decimal,
    Text,
    Bytes,
    Date,
    Time,
    DateTime,
    Json,
    Uuid,
    Array,
    Other,
}

/// 列的完整类型信息
#[derive(Debug, Clone)]
pub struct DbTypeInfo {
    pub family: DbTypeFamily,
    /// 数据库原生类型名（如 "INTEGER", "character varying(255)", "bigint"）
    pub native_name: String,
    /// 是否可为 NULL（None = 未知）
    pub nullable: Option<bool>,
}

// ── DbValue 方法 ──

impl DbValue {
    /// 人类可读的显示文本
    pub fn display(&self) -> String {
        match self {
            Self::Null => "NULL".to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Int(i) => i.to_string(),
            Self::UInt(u) => u.to_string(),
            Self::Float(f) => {
                // 避免科学记数法对常见值的干扰
                if f.is_nan() {
                    "NaN".to_string()
                } else if f.is_infinite() {
                    if *f > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else {
                    // 对于"看起来像整数"的浮点数，保留一位小数以区分类别
                    let s = f.to_string();
                    if s.contains('.') || s.contains('e') {
                        s
                    } else {
                        format!("{:.1}", f)
                    }
                }
            }
            Self::Decimal(s) => s.clone(),
            Self::Text(s) => s.clone(),
            Self::Bytes(b) => format!("<{} bytes>", b.len()),
            Self::Date(d) => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
            Self::Time(t) => format!("{:02}:{:02}:{:02}", t.hour, t.minute, t.second),
            Self::DateTime(dt) => format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                dt.date.year,
                dt.date.month,
                dt.date.day,
                dt.time.hour,
                dt.time.minute,
                dt.time.second,
            ),
            Self::Json(j) => j.to_string(),
            Self::Uuid(u) => u.to_string(),
            Self::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Self::Other { display, .. } => display.clone(),
        }
    }

    /// 类型感知的语义比较。
    ///
    /// 用于排序和筛选。不同类型族之间的比较顺序：
    /// Null < Bool < Int/UInt/Float/Decimal < Text < Bytes < Date/Time/DateTime < Json < Array < Other
    pub fn cmp_semantic(&self, other: &DbValue) -> Ordering {
        use DbValue::*;
        match (self, other) {
            // NULL 总是最小
            (Null, Null) => Ordering::Equal,
            (Null, _) => Ordering::Less,
            (_, Null) => Ordering::Greater,

            // 数值族：按 f64 比较（对 Decimal 做解析，失败则回退到字典序）
            (Int(a), Int(b)) => a.cmp(b),
            (Int(a), UInt(b)) => (*a as i128).cmp(&(*b as i128)),
            (Int(a), Float(b)) => compare_f64(*a as f64, *b),
            (Int(a), Decimal(b)) => compare_decimal(&a.to_string(), b),
            (UInt(a), Int(b)) => (*a as i128).cmp(&(*b as i128)),
            (UInt(a), UInt(b)) => a.cmp(b),
            (UInt(a), Float(b)) => compare_f64(*a as f64, *b),
            (UInt(a), Decimal(b)) => compare_decimal(&a.to_string(), b),
            (Float(a), Int(b)) => compare_f64(*a, *b as f64),
            (Float(a), UInt(b)) => compare_f64(*a, *b as f64),
            (Float(a), Float(b)) => compare_f64(*a, *b),
            (Float(a), Decimal(b)) => compare_decimal(&a.to_string(), b),
            (Decimal(a), Int(b)) => compare_decimal(a, &b.to_string()),
            (Decimal(a), UInt(b)) => compare_decimal(a, &b.to_string()),
            (Decimal(a), Float(b)) => compare_decimal(a, &b.to_string()),
            (Decimal(a), Decimal(b)) => compare_decimal(a, b),

            // 文本：字典序
            (Text(a), Text(b)) => a.cmp(b),

            // 布尔：false < true
            (Bool(a), Bool(b)) => a.cmp(b),

            // 二进制：按长度再按内容
            (Bytes(a), Bytes(b)) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),

            // 日期时间族
            (Date(a), Date(b)) => date_cmp(a, b),
            (Time(a), Time(b)) => time_cmp(a, b),
            (DateTime(a), DateTime(b)) => datetime_cmp(a, b),
            (Date(a), DateTime(b)) => datetime_cmp(&to_datetime(*a, DbTime::midnight()), b),
            (DateTime(a), Date(b)) => datetime_cmp(a, &to_datetime(*b, DbTime::midnight())),

            // JSON：按字符串表示比较
            (Json(a), Json(b)) => a.to_string().cmp(&b.to_string()),

            // UUID：字典序
            (Uuid(a), Uuid(b)) => a.to_string().cmp(&b.to_string()),

            // Array：先按长度再按元素逐一比较
            (Array(a), Array(b)) => a.len().cmp(&b.len()).then_with(|| {
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| x.cmp_semantic(y))
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            }),

            // Other：按显示文本比较
            (Other { display: a, .. }, Other { display: b, .. }) => a.cmp(b),

            // 跨族比较：按族序
            (a, b) => family_order(a).cmp(&family_order(b)),
        }
    }
}

/// 类型族的排序权重（低 = 排前面）
fn family_order(v: &DbValue) -> u8 {
    match v {
        DbValue::Null => 0,
        DbValue::Bool(_) => 1,
        DbValue::Int(_) | DbValue::UInt(_) | DbValue::Float(_) | DbValue::Decimal(_) => 2,
        DbValue::Text(_) => 3,
        DbValue::Bytes(_) => 4,
        DbValue::Date(_) | DbValue::Time(_) | DbValue::DateTime(_) => 5,
        DbValue::Json(_) => 6,
        DbValue::Uuid(_) => 7,
        DbValue::Array(_) => 8,
        DbValue::Other { .. } => 9,
    }
}

fn compare_f64(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater, // NaN 排最后
        (false, true) => Ordering::Less,
        (false, false) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
    }
}

fn compare_decimal(a: &str, b: &str) -> Ordering {
    // 尝试解析为 f64 比较，失败回退到字典序
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(a), Ok(b)) => compare_f64(a, b),
        _ => a.cmp(b),
    }
}

fn date_cmp(a: &DbDate, b: &DbDate) -> Ordering {
    a.year
        .cmp(&b.year)
        .then_with(|| a.month.cmp(&b.month))
        .then_with(|| a.day.cmp(&b.day))
}

fn time_cmp(a: &DbTime, b: &DbTime) -> Ordering {
    a.hour
        .cmp(&b.hour)
        .then_with(|| a.minute.cmp(&b.minute))
        .then_with(|| a.second.cmp(&b.second))
        .then_with(|| a.nanos.cmp(&b.nanos))
}

fn datetime_cmp(a: &DbDateTime, b: &DbDateTime) -> Ordering {
    date_cmp(&a.date, &b.date).then_with(|| time_cmp(&a.time, &b.time))
}

fn to_datetime(date: DbDate, time: DbTime) -> DbDateTime {
    DbDateTime { date, time }
}

impl DbTime {
    pub const fn midnight() -> Self {
        Self {
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        }
    }
}

// ── 序列化支持 ──

impl serde::Serialize for DbValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Int(i) => serializer.serialize_i64(*i),
            Self::UInt(u) => serializer.serialize_u64(*u),
            Self::Float(f) => serializer.serialize_f64(*f),
            Self::Decimal(s) | Self::Text(s) => serializer.serialize_str(s),
            Self::Bytes(_) => serializer.serialize_str(&self.display()),
            Self::Date(_) | Self::Time(_) | Self::DateTime(_) => {
                serializer.serialize_str(&self.display())
            }
            Self::Json(j) => j.serialize(serializer),
            Self::Uuid(u) => serializer.serialize_str(&u.to_string()),
            Self::Array(arr) => arr.serialize(serializer),
            Self::Other { display, .. } => serializer.serialize_str(display),
        }
    }
}
