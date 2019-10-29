use actix_web::web;
use actix_web::Error as AWError;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Duration, Utc};
use diesel;
use diesel::prelude::*;
use diesel::r2d2;
use futures::future::Future;
use serde::Serialize;

use std::marker::Send;

use crate::error::{Error, Result};
use crate::models::{ApproxF32, LiquidVolume, TimePeriod};

pub type Pool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type Connection = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

pub trait Query {
    type Result: Send;

    fn execute(&self, conn: Connection) -> Self::Result;
}

pub fn execute<T: Query + Send + 'static>(
    pool: &Pool,
    query: T,
) -> impl Future<Item = T::Result, Error = Error> {
    let pool = pool.clone();

    web::block::<_, _, Error>(move || Ok(query.execute(pool.get()?))).from_err()
}

#[derive(Queryable, Serialize)]
pub struct Entry {
    pub id: i32,
    pub drank_on: NaiveDate,
    pub time: TimePeriod,
    pub drink_id: i32,
    pub name: String,

    pub min_abv: Option<ApproxF32>,
    pub max_abv: Option<ApproxF32>,
    pub multiplier: f32,

    pub min_quantity: ApproxF32,
    pub max_quantity: ApproxF32,

    pub volume: Option<LiquidVolume>,
    pub volume_ml: Option<LiquidVolume>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/*************************************/
/** Get Drinks query                **/
/*************************************/

#[derive(Clone)]
pub struct GetDrinks {
    pub person_id: i32,
}

impl Query for GetDrinks {
    type Result = Result<Vec<Entry>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use crate::schema::drink;
        use crate::schema::drink::dsl::*;
        use crate::schema::entry;
        use crate::schema::entry::dsl::*;

        Ok(entry
            .inner_join(drink)
            .select((
                entry::id,
                entry::drank_on,
                entry::time_period,
                entry::drink_id,
                drink::name,
                drink::min_abv,
                drink::max_abv,
                drink::multiplier,
                entry::min_quantity,
                entry::max_quantity,
                entry::volume,
                entry::volume_ml,
                entry::created_at,
                entry::updated_at,
            ))
            .filter(entry::person_id.eq(&self.person_id))
            .order(entry::drank_on.asc())
            .then_order_by(entry::time_period.asc())
            .load::<Entry>(&conn)?)
    }
}
