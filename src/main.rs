#![feature(option_result_contains)]

#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

mod import;

use import::{DateContext, Drink, DrinkSet, QuantityRange, RawEntry, VolumeUnit};

fn main() -> std::io::Result<()> {
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

        let id = drink_set.get_id(&drink);

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
