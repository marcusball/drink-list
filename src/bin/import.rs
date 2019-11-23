use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;

use drink_list::import::{DateContext, Drink, DrinkSet, QuantityRange, RawEntry, VolumeContext};
use drink_list::models::TimePeriod;
use drink_list::{models, schema};

fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");

    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}!", database_url))
}

fn create_drink(conn: &PgConnection, drink: &Drink) -> models::Drink {
    use models::ApproxF32;
    use schema::drink;

    let new_drink = models::NewDrink {
        name: drink.name.as_str(),

        min_abv: drink.abv.as_ref().map(|abv| abv.min),
        max_abv: drink.abv.as_ref().map(|abv| abv.max),

        multiplier: drink.multiplier,
    };

    diesel::insert_into(drink::table)
        .values(&new_drink)
        .get_result(conn)
        .expect("Error saving new drink")
}

fn create_entry(
    conn: &PgConnection,
    drink_id: i32,
    date: &DateContext,
    quantity: &QuantityRange,
    volume: &Option<VolumeContext>,
) -> models::PlainEntry {
    use models::*;
    use schema::entry;
    use uom::si::volume::{centiliter, fluid_ounce, liter, milliliter};

    let new_entry = models::NewEntry {
        person_id: 1,
        drank_on: &date.date,
        time_period: &date.time,
        context: &date.context,
        drink_id: drink_id,
        min_quantity: &quantity.min,
        max_quantity: &quantity.max,
        volume: volume.clone().as_ref().map(|v| v.volume),
        volume_ml: volume.clone().as_ref().map(|v| v.volume.to_ml()),
    };

    diesel::insert_into(entry::table)
        .values(&new_entry)
        .get_result(conn)
        .expect("Error saving new entry")
}

fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_conn = establish_connection();

    let f = File::open("drinks.csv")?;
    let mut reader = BufReader::new(f);

    let mut line = String::new();

    let mut previous_date = DateContext {
        date: chrono::NaiveDate::from_ymd(2018, 1, 1),
        time: TimePeriod::Evening,
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
        let volume = VolumeContext::from_entry(&entry);

        let id = match drink_set.find(&drink) {
            Some(id) => id,
            None => {
                let db_drink = create_drink(&db_conn, &drink);
                drink_set.insert(db_drink.id, drink.clone())
            }
        };

        create_entry(&db_conn, id, &date, &quantity, &volume);

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
