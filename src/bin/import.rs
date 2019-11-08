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

    // This seems like an obvious desire, but I can't figure out
    // how to get uom to give me the value in the original units.
    let get_volume = |v: &VolumeContext| -> f32 {
        match v.original_unit.unwrap() {
            VolumeUnit::FlOz => v.value.get::<fluid_ounce>(),
            VolumeUnit::mL => v.value.get::<milliliter>(),
            VolumeUnit::cL => v.value.get::<centiliter>(),
            VolumeUnit::L => v.value.get::<liter>(),
        }
    };

    let new_entry = models::NewEntry {
        person_id: 1,
        drank_on: &date.date,
        time_period: &date.time,
        drink_id: drink_id,
        min_quantity: &ApproxF32 {
            num: quantity.min,
            is_approximate: quantity.approximate_min,
        },
        max_quantity: &ApproxF32 {
            num: quantity.max,
            is_approximate: quantity.approximate_max,
        },
        volume: volume.clone().as_ref().map(|v| models::LiquidVolume {
            amount: models::ApproxF32 {
                num: get_volume(v),
                is_approximate: v.approximate,
            },
            unit: v.original_unit.unwrap(),
        }),
        volume_ml: volume.clone().as_ref().map(|v| models::LiquidVolume {
            amount: models::ApproxF32 {
                num: v.value.get::<milliliter>(),
                is_approximate: v.approximate,
            },
            unit: VolumeUnit::mL,
        }),
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
