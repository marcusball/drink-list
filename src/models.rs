use crate::schema::Volume as DbVolume;
use crate::schema::*;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql, WriteTuple};
use diesel::sql_types::{Bool, Float4, Record};
use std::io::Write;

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "Realapprox"]
pub struct ApproxF32 {
    pub num: f32,
    pub is_approximate: bool,
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "Timeperiod"]
pub enum TimePeriod {
    Morning,
    Afternoon,
    Evening,
    Night,
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "Volumeunit"]
#[allow(non_camel_case_types)]
pub enum VolumeUnit {
    FlOz,
    mL,
    cL,
    L,
}

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "DbVolume"]
pub struct Volume(ApproxF32, VolumeUnit);

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
        match *self {
            TimePeriod::Morning => out.write_all(b"morning")?,
            TimePeriod::Afternoon => out.write_all(b"afternoon")?,
            TimePeriod::Evening => out.write_all(b"evening")?,
            TimePeriod::Night => out.write_all(b"night")?,
        }
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

impl ToSql<DbVolume, Pg> for Volume {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        WriteTuple::<(Realapprox, Volumeunit)>::write_tuple(&(&self.0, &self.1), out)
    }
}

impl FromSql<DbVolume, Pg> for Volume {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let (vol, unit) = FromSql::<Record<(Realapprox, Volumeunit)>, Pg>::from_sql(bytes)?;
        Ok(Volume(vol, unit))
    }
}

#[derive(Queryable)]
pub struct Entry {
    pub id: i32,
    pub person_id: i32,
    pub drank_on: NaiveDate,
    pub time: String,
    pub drink_id: i32,
    pub name: String,

    pub min_abv: Option<ApproxF32>,
    pub max_abv: Option<ApproxF32>,
    pub multiplier: f32,

    pub min_quantity: ApproxF32,
    pub max_quantity: ApproxF32,

    pub volume: Option<Volume>,
    pub volume_ml: Option<Volume>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
