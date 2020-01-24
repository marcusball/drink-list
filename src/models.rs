use crate::schema::*;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql, WriteTuple};
use diesel::sql_types::{Bool, Float4, Record};
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::io::Write;
use uom::si::f32::Volume as SiVolume;

/// What percentage +/- should be applied to approximate values.
static APPROX_MODIFIER: f32 = 0.1;

#[derive(Clone, Copy, Debug, FromSqlRow, AsExpression, Serialize, PartialEq, QueryId)]
#[sql_type = "Realapprox"]
pub struct ApproxF32 {
    pub num: f32,
    pub is_approximate: bool,
}

impl ApproxF32 {
    pub fn new(num: f32, is_approximate: bool) -> ApproxF32 {
        ApproxF32 {
            num,
            is_approximate,
        }
    }

    #[inline]
    pub fn min(&self) -> f32 {
        // This is a (probably dumb, unnecessary) attempt to avoid a conditional
        // so as to just use pure math operations.
        // In pseudocode, this is: `abv.is_approximate ? abv.num * (1 - MOD) : abv.num`.
        self.num
            * (1.0
                - (APPROX_MODIFIER
                    + ((!self.is_approximate as i32) as f32 * -1.0 * APPROX_MODIFIER)))
    }

    #[inline]
    pub fn max(&self) -> f32 {
        // This is a (probably dumb, unnecessary) attempt to avoid a conditional
        // so as to just use pure math operations.
        // In pseudocode, this is: `abv.is_approximate ? abv.num * (1 + MOD) : abv.num`.
        self.num
            * (1.0
                + (APPROX_MODIFIER
                    + ((!self.is_approximate as i32) as f32 * -1.0 * APPROX_MODIFIER)))
    }

    /// Increment this value by one.
    pub fn increment(&mut self) {
        self.num = self.num + 1.0;
    }
}

impl Hash for ApproxF32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((self.num * 100.0).trunc() as i32).hash(state);
        self.is_approximate.hash(state);
    }
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
pub struct LiquidVolume {
    pub amount: ApproxF32,
    pub unit: VolumeUnit,
}

impl LiquidVolume {
    pub fn to_si_volume(&self) -> SiVolume {
        use uom::si::volume::{centiliter, fluid_ounce, liter, milliliter};

        match self.unit {
            VolumeUnit::FlOz => SiVolume::new::<fluid_ounce>(self.amount.num),
            VolumeUnit::mL => SiVolume::new::<milliliter>(self.amount.num),
            VolumeUnit::cL => SiVolume::new::<centiliter>(self.amount.num),
            VolumeUnit::L => SiVolume::new::<liter>(self.amount.num),
        }
    }

    pub fn to_ml(&self) -> LiquidVolume {
        use uom::si::volume::milliliter;

        let ml = self.to_si_volume().get::<milliliter>();
        let mut amount = self.amount.clone();
        amount.num = ml;

        LiquidVolume {
            unit: VolumeUnit::mL,
            amount: amount,
        }
    }
}

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
        WriteTuple::<(Realapprox, Volumeunit)>::write_tuple(&(&self.amount, &self.unit), out)
    }
}

impl FromSql<Volume, Pg> for LiquidVolume {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (vol, unit) = FromSql::<Record<(Realapprox, Volumeunit)>, Pg>::from_sql(bytes)?;
        Ok(LiquidVolume {
            amount: vol,
            unit: unit,
        })
    }
}

#[derive(Queryable)]
pub struct PlainEntry {
    pub id: i32,
    pub person_id: i32,
    pub drank_on: NaiveDate,
    pub time: TimePeriod,
    pub context: Vec<String>,
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
    pub context: &'a Vec<String>,
    pub drink_id: i32,
    pub min_quantity: &'a ApproxF32,
    pub max_quantity: &'a ApproxF32,
    pub volume: Option<LiquidVolume>,
    pub volume_ml: Option<LiquidVolume>,
}

#[derive(Queryable, Debug)]
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
