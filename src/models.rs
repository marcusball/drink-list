use crate::schema::*;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql, WriteTuple};
use diesel::sql_types::{Bool, Float4, Record};
use std::io::Write;

#[derive(Debug, FromSqlRow, AsExpression)]
#[sql_type = "Realapprox"]
pub struct ApproxF32 {
    pub num: f32,
    pub is_approximate: bool,
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

    pub volume: Option<ApproxF32>,
    pub volume_unit: Option<String>,

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
