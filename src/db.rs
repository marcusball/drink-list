use actix_web::web;
use actix_web::Error as AWError;
use chrono::naive::NaiveDate;
use chrono::{DateTime, Duration, Utc};
use diesel;
use diesel::prelude::*;
use diesel::r2d2;
use diesel::sql_types::Text;
use futures::future::Future;
use serde::Serialize;

use std::marker::Send;

use crate::error::{Error, Result};
use crate::import::{Abv, QuantityRange, VolumeContext};
use crate::models;
use crate::models::{ApproxF32, Drink, LiquidVolume, TimePeriod};
use crate::schema;

pub type Pool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type Connection = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;

// Diesel does not have a `lower` function built in; create one ourselves.
// See: https://github.com/diesel-rs/diesel/issues/560#issuecomment-270199166
sql_function!(fn lower(x: Text) -> Text);

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
    pub context: Vec<String>,
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

impl Entry {
    #[inline]
    pub fn min_quantity(&self) -> f32 {
        self.min_quantity.min()
    }

    #[inline]
    pub fn max_quantity(&self) -> f32 {
        self.max_quantity.max()
    }

    /// Get the min ABV range as a float
    pub fn min_abv(&self) -> Option<f32> {
        self.min_abv.map(|abv| abv.min())
    }

    /// Get the max ABV range as a float
    pub fn max_abv(&self) -> Option<f32> {
        self.max_abv.map(|abv| abv.max())
    }

    /// Check if this entry has any ABV information.
    pub fn has_abv(&self) -> bool {
        // Either both or neither should be present.
        assert_eq!(self.min_abv.is_some(), self.max_abv.is_some());

        // Given the assertion, only going to check min.
        self.min_abv.is_some()
    }

    /// Check if this entry has any volume information.
    pub fn has_volume(&self) -> bool {
        self.volume.is_some()
    }
}

/*************************************/
/** Get Drinks query                **/
/*************************************/

#[derive(Clone)]
pub struct GetDrinks {
    pub person_id: i32,
    pub date_range: Option<(NaiveDate, NaiveDate)>,
}

impl Query for GetDrinks {
    type Result = Result<Vec<Entry>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use crate::schema::drink;
        use crate::schema::drink::dsl::*;
        use crate::schema::entry;
        use crate::schema::entry::dsl::*;

        /* let filter = match self.date_range {
            Some((start, end)) => Box::new(
                entry::person_id
                    .eq(&self.person_id)
                    .and(entry::drank_on.ge(start))
                    .and(entry::drank_on.le(end)),
            ),
            None => Box::new(entry::person_id.eq(&self.person_id)),
        };*/

        let mut query = entry
            .inner_join(drink)
            .select((
                entry::id,
                entry::drank_on,
                entry::time_period,
                entry::context,
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
            .into_boxed();

        if let Some((start, end)) = self.date_range {
            query = query.filter(entry::drank_on.ge(start).and(entry::drank_on.le(end)));
        }
        Ok(query
            .order(entry::drank_on.asc())
            .then_order_by(entry::time_period.asc())
            .load::<Entry>(&conn)?)
    }
}

/*************************************/
/** Get Entry query                 **/
/*************************************/

#[derive(Clone)]
pub struct GetEntry {
    pub person_id: i32,
    pub entry_id: i32,
}

impl Query for GetEntry {
    type Result = Result<Option<Entry>>;

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
                entry::context,
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
            .filter(
                entry::person_id
                    .eq(&self.person_id)
                    .and(entry::id.eq(&self.entry_id)),
            )
            .first::<Entry>(&conn)
            .optional()?)
    }
}

/*************************************/
/*************************************/

pub struct GetDrink {
    // @TODO: Associate with person ID?
    pub name: String,
    pub abv: Option<Abv>,
}

impl Query for GetDrink {
    type Result = Result<Option<Drink>>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::drink::dsl::*;

        let min = self.abv.as_ref().map(|abv| abv.min);
        let max = self.abv.as_ref().map(|abv| abv.max);

        Ok(drink
            .filter(
                lower(name)
                    .eq(&self.name.to_lowercase())
                    .and(min_abv.eq(&min))
                    .and(max_abv.eq(&max)),
            )
            .first::<Drink>(&conn)
            .optional()?)
    }
}

/*************************************/
/*************************************/

pub struct CreateDrink {
    pub name: String,
    pub abv: Option<Abv>,
    pub multiplier: f32,
}

impl Query for CreateDrink {
    type Result = Result<Drink>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use super::schema::drink;

        let min = self.abv.as_ref().map(|abv| abv.min);
        let max = self.abv.as_ref().map(|abv| abv.max);

        let new_drink = super::models::NewDrink {
            name: self.name.as_str(),

            min_abv: min,
            max_abv: max,

            multiplier: self.multiplier,
        };

        Ok(diesel::insert_into(drink::table)
            .values(&new_drink)
            .get_result(&conn)?)
    }
}

/*************************************/
/*************************************/

pub struct CreateEntry {
    pub person_id: i32,
    pub drank_on: NaiveDate,
    pub time_period: models::TimePeriod,
    pub context: Vec<String>,
    pub drink_id: i32,
    pub quantity: QuantityRange,
    pub volume: Option<VolumeContext>,
}

impl Query for CreateEntry {
    type Result = Result<models::PlainEntry>;

    fn execute(&self, conn: Connection) -> Self::Result {
        use schema::entry;

        let new_entry = models::NewEntry {
            person_id: self.person_id,
            drank_on: &self.drank_on,
            time_period: &self.time_period,
            context: &self.context,
            drink_id: self.drink_id,
            min_quantity: &self.quantity.min,
            max_quantity: &self.quantity.max,
            volume: self.volume.as_ref().map(|v| v.volume),
            volume_ml: self.volume.as_ref().map(|v| v.volume.to_ml()),
        };

        Ok(diesel::insert_into(entry::table)
            .values(&new_entry)
            .get_result(&conn)?)
    }
}
