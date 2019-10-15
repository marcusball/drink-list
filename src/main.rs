#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

mod import;

use import::RawEntry;

fn main() -> std::io::Result<()> {
    let f = File::open("drinks.csv")?;
    let mut reader = BufReader::new(f);

    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        let entry = RawEntry::from_line(&line.trim());

        match entry {
            Some(entry) => println!(
                "{:16} | {:10} | {:40} | {:5} | {:10}",
                entry.date.unwrap_or("".into()),
                entry.quantity.unwrap_or("?".into()),
                entry.name.unwrap_or("####".into()),
                entry.abv.unwrap_or("?".into()),
                entry.volume.unwrap_or("?".into())
            ),
            None => println!("ERROR: Failed to parse '{}'", line),
        }

        line.clear();
    }

    Ok(())
}
