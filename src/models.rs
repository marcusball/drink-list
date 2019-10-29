use crate::schema::*;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql, WriteTuple};
use diesel::sql_types::{Bool, Float4, Record};
use serde::Serialize;
use std::io::Write;

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression, Serialize)]
#[sql_type = "Realapprox"]
pub struct ApproxF32 {
    pub num: f32,
    pub is_approximate: bool,
}

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression, Serialize)]
#[sql_type = "Timeperiod"]
pub enum TimePeriod {
    Morning,
    Afternoon,
    Evening,
    Night,
}

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression, Serialize)]
#[sql_type = "Volumeunit"]
#[allow(non_camel_case_types)]
pub enum VolumeUnit {
    FlOz,
    mL,
    cL,
    L,
}

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression, Serialize)]
#[sql_type = "Volume"]
pub struct LiquidVolume(pub ApproxF32, pub VolumeUnit);

impl TimePeriod {
    /// Returns whether the given `time` string is a recognized time period.
    pub fn is_time_string(time: &str) -> bool {
        Self::from_str(time).is_some()
    }

    pub fn from_str(time: &str) -> Option<TimePeriod> {
        match time {
            "morning" => Some(TimePeriod::Morning),
            "afternoon" => Some(TimePeriod::Afternoon),
            "evening" => Some(TimePeriod::Evening),
            "night" => Some(TimePeriod::Night),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            TimePeriod::Morning => "morning",
            TimePeriod::Afternoon => "afternoon",
            TimePeriod::Evening => "evening",
            TimePeriod::Night => "night",
        }
    }
}

impl std::fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl ToSql<Realapprox, Pg> for ApproxF32 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        WriteTuple::<(Float4, Bool)>::write_tuple(&(self.num, self.is_approximate), out)
    }
}

impl FromSql<Realapprox, Pg> for ApproxF32 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (num, is_approximate) = FromSql::<Record<(Float4, Bool)>, Pg>::from_sql(bytes)?;
        Ok(ApproxF32 {
            num,
            is_approximate,
        })
    }
}

impl ToSql<Timeperiod, Pg> for TimePeriod {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(self.to_str().as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Timeperiod, Pg> for TimePeriod {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"morning" => Ok(TimePeriod::Morning),
            b"afternoon" => Ok(TimePeriod::Afternoon),
            b"evening" => Ok(TimePeriod::Evening),
            b"night" => Ok(TimePeriod::Night),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl VolumeUnit {
    pub fn from_str(unit: &str) -> Option<VolumeUnit> {
        match unit.to_lowercase().as_str() {
            "fl oz" | "oz" => Some(VolumeUnit::FlOz),
            "ml" => Some(VolumeUnit::mL),
            "cl" => Some(VolumeUnit::cL),
            "l" => Some(VolumeUnit::L),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            VolumeUnit::FlOz => "fl oz",
            VolumeUnit::mL => "mL",
            VolumeUnit::cL => "cL",
            VolumeUnit::L => "L",
        }
    }
}

impl ToSql<Volumeunit, Pg> for VolumeUnit {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            VolumeUnit::FlOz => out.write_all(b"fl oz")?,
            VolumeUnit::mL => out.write_all(b"mL")?,
            VolumeUnit::cL => out.write_all(b"cL")?,
            VolumeUnit::L => out.write_all(b"L")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Volumeunit, Pg> for VolumeUnit {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"fl oz" => Ok(VolumeUnit::FlOz),
            b"mL" => Ok(VolumeUnit::mL),
            b"cL" => Ok(VolumeUnit::cL),
            b"L" => Ok(VolumeUnit::L),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl std::fmt::Display for VolumeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl ToSql<Volume, Pg> for LiquidVolume {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        WriteTuple::<(Realapprox, Volumeunit)>::write_tuple(&(&self.0, &self.1), out)
    }
}

impl FromSql<Volume, Pg> for LiquidVolume {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (vol, unit) = FromSql::<Record<(Realapprox, Volumeunit)>, Pg>::from_sql(bytes)?;
        Ok(LiquidVolume(vol, unit))
    }
}

#[derive(Queryable)]
pub struct PlainEntry {
    pub id: i32,
    pub person_id: i32,
    pub drank_on: NaiveDate,
    pub time: TimePeriod,
    pub drink_id: i32,

    pub min_quantity: ApproxF32,
    pub max_quantity: ApproxF32,

    pub volume: Option<LiquidVolume>,
    pub volume_ml: Option<LiquidVolume>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "entry"]
pub struct NewEntry<'a> {
    pub person_id: i32,
    pub drank_on: &'a NaiveDate,
    pub time_period: &'a TimePeriod,
    pub drink_id: i32,
    pub min_quantity: &'a ApproxF32,
    pub max_quantity: &'a ApproxF32,
    pub volume: Option<LiquidVolume>,
    pub volume_ml: Option<LiquidVolume>,
}

#[derive(Queryable)]
pub struct Drink {
    pub id: i32,
    pub name: String,

    pub min_abv: Option<ApproxF32>,
    pub max_abv: Option<ApproxF32>,
    pub multiplier: f32,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "drink"]
pub struct NewDrink<'a> {
    pub name: &'a str,
    pub min_abv: Option<ApproxF32>,
    pub max_abv: Option<ApproxF32>,
    pub multiplier: f32,
}
