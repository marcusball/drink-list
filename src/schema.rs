#![allow(unused_imports)]

#[derive(Debug, SqlType, QueryId)]
#[postgres(type_name = "realapprox")]
pub struct Realapprox;
#[derive(Debug, SqlType)]
#[postgres(type_name = "timeperiod")]
pub struct Timeperiod;

#[derive(Debug, SqlType)]
#[postgres(type_name = "volumeunit")]
pub struct Volumeunit;

#[derive(Debug, SqlType)]
#[postgres(type_name = "volume")]
pub struct Volume;

table! {
    use diesel::sql_types::*;
    use super::{Realapprox, Timeperiod, Volumeunit, Volume};

    drink (id) {
        id -> Int4,
        name -> Varchar,
        min_abv -> Nullable<Realapprox>,
        max_abv -> Nullable<Realapprox>,
        multiplier -> Float4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::*;
    use super::{Realapprox, Timeperiod, Volumeunit, Volume};

    entry (id) {
        id -> Int4,
        person_id -> Int4,
        drank_on -> Date,
        time_period -> Timeperiod,
        context -> Array<Text>,
        drink_id -> Int4,
        min_quantity -> Realapprox,
        max_quantity -> Realapprox,
        volume -> Nullable<Volume>,
        volume_ml -> Nullable<Volume>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::*;
    use super::{Realapprox, Timeperiod, Volumeunit, Volume};

    person (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

joinable!(entry -> drink (drink_id));
joinable!(entry -> person (person_id));

allow_tables_to_appear_in_same_query!(drink, entry, person,);
