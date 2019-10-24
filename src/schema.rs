#[derive(Debug, SqlType)]
#[postgres(type_name = "realapprox")]
pub struct Realapprox;

table! {
    use diesel::sql_types::*;
    use super::Realapprox;

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
    use super::Realapprox;

    entry (id) {
        id -> Int4,
        person_id -> Int4,
        drank_on -> Date,
        time_id -> Int4,
        drink_id -> Int4,
        min_quantity -> Realapprox,
        max_quantity -> Realapprox,
        volume -> Nullable<Realapprox>,
        volume_unit_id -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::*;
    use super::Realapprox;

    person (id) {
        id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::*;
    use super::Realapprox;

    time_period (id) {
        id -> Int4,
        name -> Varchar,
    }
}

table! {
    use diesel::sql_types::*;
    use super::Realapprox;

    volume_unit (id) {
        id -> Int4,
        abbr -> Varchar,
    }
}

joinable!(entry -> drink (drink_id));
joinable!(entry -> person (person_id));
joinable!(entry -> time_period (time_id));
joinable!(entry -> volume_unit (volume_unit_id));

allow_tables_to_appear_in_same_query!(
    drink,
    entry,
    person,
    time_period,
    volume_unit,
);
