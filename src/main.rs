#![feature(option_result_contains)]

#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

mod import;

use import::{Date, RawEntry};

fn main() -> std::io::Result<()> {
    let f = File::open("drinks.csv")?;
    let mut reader = BufReader::new(f);

    let mut line = String::new();

    let mut previous_date = Date {
        day: "1 jan".into(),
        time: "".into(),
        context: vec![],
    };

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

        let date = Date::from_entry(&entry, &previous_date);
        previous_date = date.clone();

        println!(
            "{:6} | {:9} | {:10} | {:10} | {:40} | {:5} | {:10}",
            date.day,
            date.time,
            date.context.join(", "),
            entry.quantity.unwrap_or("?".into()),
            entry.name.unwrap_or("####".into()),
            entry.abv.unwrap_or("?".into()),
            entry.volume.unwrap_or("?".into())
        );

        line.clear();
    }

    Ok(())
}
