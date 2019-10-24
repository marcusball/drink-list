#![feature(option_result_contains)]
#![feature(option_expect_none)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate diesel;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;

mod import;
mod models;
mod schema;

use import::{DateContext, Drink, DrinkSet, QuantityRange, RawEntry, VolumeUnit};

fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");

    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}!", database_url))
}

fn create_drink(conn: &PgConnection, drink: &Drink) -> models::Drink {
    use models::ApproxF32;
    use schema::drink;

    let new_drink = models::NewDrink {
        name: drink.name.as_str(),

        min_abv: drink.abv.as_ref().map(|abv| ApproxF32 {
            num: abv.min,
            is_approximate: abv.approximate_min,
        }),
        max_abv: drink.abv.as_ref().map(|abv| ApproxF32 {
            num: abv.max,
            is_approximate: abv.approximate_max,
        }),

        multiplier: 1.0,
    };

    diesel::insert_into(drink::table)
        .values(&new_drink)
        .get_result(conn)
        .expect("Error saving new drink")
}

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_conn = establish_connection();

    let f = File::open("drinks.csv")?;
    let mut reader = BufReader::new(f);

    let mut line = String::new();

    let mut previous_date = DateContext {
        date: chrono::NaiveDate::from_ymd(2018, 1, 1),
        time: "".into(),
        context: vec![],
    };

    let mut drink_set = DrinkSet::new();

    while reader.read_line(&mut line)? > 0 {
        let entry = RawEntry::from_line(&line.trim());

        let entry = match entry {
            Some(e) => e,
            None => {
                println!("ERROR: Failed to parse '{}'", line);
                line.clear();
                continue;
            }
        };

        let date = DateContext::from_entry(&entry, &previous_date);
        previous_date = date.clone();

        let drink = Drink::from_entry(&entry);
        let quantity = QuantityRange::from_entry(&entry);
        let volume = VolumeUnit::from_entry(&entry);

        let id = match drink_set.find(&drink) {
            Some(id) => id,
            None => {
                let db_drink = create_drink(&db_conn, &drink);
                drink_set.insert(db_drink.id, drink.clone())
            }
        };

        println!(
            "{:11} | {:9} | {:10} | {:10} | ({:3}) {:40} | {:5} | {:10}",
            date.date.format("%d %b %Y"),
            date.time,
            date.context.join(", "),
            quantity.print(),
            id,
            drink.name,
            drink.abv.map(|a| a.print()).unwrap_or("".into()),
            volume.map(|v| v.print()).unwrap_or("".into())
        );

        line.clear();
    }

    Ok(())
}
