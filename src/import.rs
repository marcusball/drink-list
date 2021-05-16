use crate::error::Error;
use crate::models::{ApproxF32, LiquidVolume, TimePeriod, VolumeUnit};
use crate::Result;
use chrono::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::hash::{Hash, Hasher};
use uom::si::f32::*;
use uom::si::volume::{centiliter, fluid_ounce, liter, milliliter};

/// Represents the components of an entry line
pub struct RawEntry {
    pub date: Option<String>,
    pub quantity: Option<String>,
    pub name: Option<String>,
    pub abv: Option<String>,
    pub volume: Option<String>,
}

impl RawEntry {
    pub fn from_line(line: &str) -> Option<RawEntry> {
        lazy_static! {
            static ref RE: Regex = Regex::new("(?:\\((?P<date>.*?)\\))?,?(?P<quantity>.*?),(?P<name>.*?)(?:,(?P<abv>.*?)(?:,(?P<volume>.*?))?)?$").unwrap();
        }

        let captures = match RE.captures(line) {
            Some(c) => c,
            None => {
                return None;
            }
        };

        let cap_str = |name| captures.name(name).map(|m| String::from(m.as_str().trim()));

        Some(RawEntry {
            date: cap_str("date"),
            quantity: cap_str("quantity"),
            name: cap_str("name"),
            abv: cap_str("abv"),
            volume: cap_str("volume"),
        })
    }
}

#[derive(Clone, Debug)]
pub struct DateContext {
    pub date: NaiveDate,
    pub time: TimePeriod,
    pub context: Vec<String>,
}

impl DateContext {
    pub fn from_entry(entry: &RawEntry, previous: &DateContext) -> DateContext {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r#"^(?P<day>(?:\d{1,2}\s\w{3})|(?:\w{3}\s\d{1,2}))?[,; ]*(?:(?P<context2>[^\r\n;,]*?)[;,]?)?(?:(?P<context1>[^\r\n;,]*?)[;,]?)?$"#
            )
            .unwrap();
        }
        if entry.date.is_none() {
            return previous.clone();
        }

        // Evaluate the regex and find any captures
        let captures = RE.captures(entry.date.as_ref().unwrap()).unwrap();

        // Helper function to retrieve matches by name, as an Option<String>
        let cap_str = |name| {
            captures
                .name(name)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
                .map(|s| s.to_lowercase())
        };

        let date = cap_str("day")
            .map(|s| Self::parse_date_string(&s, &previous.date))
            .unwrap_or(previous.date.clone());
        let context1 = cap_str("context1");
        let context2 = cap_str("context2");

        let is_time_string = |context: Option<&String>| {
            context
                .map(|c| TimePeriod::is_time_string(&c.as_ref()))
                .unwrap_or(false)
        };

        // I frequently just write "brunch"; if so we'll mark this as "afternoon".
        let is_brunch =
            Self::is_brunch_context(&context1) || Self::is_brunch_context(&context2);

        let time: TimePeriod = match (
            is_time_string(context1.as_ref()),
            is_time_string(context2.as_ref()),
        ) {
            // If one of either is a time specifier, then use that value.
            (true, false) => TimePeriod::from_str(context1.as_ref().unwrap())
                .expect("Failed to parse time period!"),
            (false, true) => TimePeriod::from_str(context2.as_ref().unwrap())
                .expect("Failed to parse time period!"),
            // If neither specify the time perioud, first check if "brunch" was present.
            (false, false) => match is_brunch {
                // If it was, then use "afternoon"
                true => TimePeriod::Afternoon,
                // Otherwise, if this record is the same day as the previous,
                // then continue using the same time as the previous.
                // Use "night" otherwise.
                false => match date == previous.date {
                    true => previous.time,
                    false => TimePeriod::Night,
                },
            },
            // There should be no case of "afternoon, night" etc.
            (true, true) => panic!(
                "Found two time strings, {} and {}!",
                context1.unwrap(),
                context2.unwrap()
            ),
        };

        let context = vec![context1, context2]
            .iter()
            // Remove any context strings that denote the time period.
            .filter(|c| c.is_some() && !TimePeriod::is_time_string(c.as_ref().unwrap()))
            .map(|c| c.as_ref().unwrap().to_string())
            .collect();

        DateContext {
            date: date,
            time: time,
            context: context,
        }
    }

    /// Parse a date string in the format "1 oct" or "feb 21".
    /// Use the `previous` date as context for inferring the proper year.
    fn parse_date_string(date: &String, previous: &NaiveDate) -> NaiveDate {
        use chrono::format::{parse, Parsed, StrftimeItems};

        // Where parsed date info will be saved
        let mut parsed = Parsed::new();

        // Parsing format for "day month" dates.
        let items = StrftimeItems::new("%b %e");

        let result = parse(&mut parsed, date.as_str(), items);

        if result.is_err() {
            parse(&mut parsed, date.as_str(), StrftimeItems::new("%e %b"))
                .expect("backup parse failed!");
        }

        let day = parsed.day.expect("Failed to parse day!");
        let month = parsed.month.expect("Failed to parse month");
        let year = match day == 1 && month == 1 {
            true => previous.year() + 1,
            false => previous.year(),
        };

        NaiveDate::from_ymd(year, month, day)
    }

    /// Test if the given time `context` is an `Option` containing "brunch".
    /// 
    /// Hacky workaround until `Option.contains()` is stabilized.
    fn is_brunch_context(context: &Option<String>) -> bool {
        match context { 
            Some (c) => {
                c.as_str() == "brunch"
            },
            None => false
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct QuantityRange {
    pub min: ApproxF32,
    pub max: ApproxF32,
}

impl QuantityRange {
    pub fn from_entry(entry: &RawEntry) -> QuantityRange {
        Self::from_str(&entry.quantity.as_ref().expect("No quantity found!")).unwrap()
    }

    pub fn from_str<S: AsRef<str>>(quantity: S) -> Result<QuantityRange> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"(~?\d+(?:\.\d+)?)(?:\s*\-\s*(~?\d+(?:\.\d+)?))?"#).unwrap();
        }

        let captures = match RE.captures(quantity.as_ref()) {
            Some(captures) => captures,
            None => {
                return Err(Error::EntryInputError("Missing required quantity!".into()));
            }
        };

        let cap_index = |index| {
            captures
                .get(index)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
        };

        let min = match cap_index(1).map(Self::parse_value) {
            Some(v) => v,
            None => {
                return Err(Error::EntryInputError("Invalid quantity input!".into()));
            }
        };
        let max = cap_index(2).map(Self::parse_value).unwrap_or(min);

        Ok(QuantityRange {
            min: ApproxF32::new(min.1, min.0),
            max: ApproxF32::new(max.1, max.0),
        })
    }

    /// Parse a strings like "2", "1.5", "~3", etc, and return a tuple
    /// indicating whether the value is approximate, and what the base numeric value is.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!((false, 1f32), QuantityRange::parse_value("1"));
    /// ```
    fn parse_value(value: &str) -> (bool, f32) {
        use std::str::FromStr;

        let is_approximate = value.starts_with("~");
        let value = f32::from_str(value.trim_start_matches("~"))
            .expect(&format!("Failed to parse number, '{}'!", value));

        (is_approximate, value)
    }

    pub fn print(&self) -> String {
        let mut display = String::new();

        if self.min.is_approximate {
            display.push_str("~");
        }

        display.push_str(&format!("{:.2}", self.min.num));

        if self.min != self.max {
            display.push('-');

            if self.max.is_approximate {
                display.push_str("~");
            }

            display.push_str(&format!("{:.2}", self.max.num));
        }

        display
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Abv {
    pub min: ApproxF32,
    pub max: ApproxF32,
}

impl Abv {
    pub fn from_entry(entry: &RawEntry) -> Option<Abv> {
        if entry.abv.is_none() {
            return None;
        }

        Self::from_str(&entry.abv.as_ref().expect("No ABV found!"))
            .expect("A minimum ABV is required!")
    }

    pub fn from_str<S: AsRef<str>>(abv: S) -> Result<Option<Abv>> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"(~?\d+(?:\.\d+)?)%?(?:\s*\-\s*(~?\d+(?:\.\d+)?)%?)?%"#).unwrap();
        }

        let captures = match RE.captures(abv.as_ref()) {
            Some(c) => c,
            None => return Ok(None),
        };

        let cap_index = |index| {
            captures
                .get(index)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
        };

        let min = match cap_index(1).map(Self::parse_value) {
            Some(v) => v,
            None => {
                return Err(Error::EntryInputError("A minimum ABV is required!".into()));
            }
        };

        let max = cap_index(2).map(Self::parse_value).unwrap_or(min);

        Ok(Some(Abv {
            min: ApproxF32::new(min.1, min.0),
            max: ApproxF32::new(max.1, max.0),
        }))
    }

    /// Parse a strings like "2", "1.5", "~3", etc, and return a tuple
    /// indicating whether the value is approximate, and what the base numeric value is.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!((false, 1f32), QuantityRange::parse_value("1"));
    /// ```
    fn parse_value(value: &str) -> (bool, f32) {
        use std::str::FromStr;

        let is_approximate = value.starts_with("~");
        let value = f32::from_str(value.trim_start_matches("~"))
            .expect(&format!("Failed to parse number, '{}'!", value));

        (is_approximate, value)
    }

    pub fn print(&self) -> String {
        let mut display = String::new();

        if self.min.is_approximate {
            display.push_str("~");
        }

        display.push_str(&format!("{:.1}", self.min.num));

        if self.min != self.max {
            display.push('-');

            if self.max.is_approximate {
                display.push_str("~");
            }

            display.push_str(&format!("{:.1}", self.max.num));
        }

        display.push('%');

        display
    }
}

pub struct VolumeContext {
    pub volume: LiquidVolume,
    pub original_unit: Option<VolumeUnit>,
}

impl VolumeContext {
    pub fn from_entry(entry: &RawEntry) -> Option<VolumeContext> {
        if entry.volume.is_none() {
            return None;
        }

        match Self::from_str(entry.volume.as_ref().unwrap()) {
            Ok(volume) => volume,
            Err(e) => {
                match e {
                    Error::EntryInputError(message) => {
                        println!("{}", message);
                    }
                    e => println!("{}", e.description()),
                };
                return None;
            }
        }
    }

    pub fn from_str<S: AsRef<str>>(volume: S) -> Result<Option<VolumeContext>> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"(?P<volume>~?\d+(?:\.\d+)?)\s*(?P<unit>\w{2,})"#).unwrap();
        }

        let captures = match RE.captures(volume.as_ref()) {
            Some(c) => c,
            None => {
                return Ok(None);
            }
        };

        // Helper function to retrieve matches by name, as an Option<String>
        let cap_str = |name| {
            captures
                .name(name)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
                .map(|s| s.to_lowercase())
        };

        let volume_str = cap_str("volume");
        let unit_str = cap_str("unit");

        if volume_str.is_none() || unit_str.is_none() {
            return Ok(None);
        }

        let (is_approximate, volume_amount) = Self::parse_value(volume_str.as_ref().unwrap());

        let unit = match VolumeUnit::from_str(unit_str.as_ref().unwrap().as_ref()) {
            Some(unit) => unit,
            None => {
                return Err(Error::EntryInputError(format!(
                    "Unrecognized volume unit, '{}'!",
                    unit_str.as_ref().unwrap()
                )));
            }
        };

        Ok(Some(VolumeContext {
            volume: LiquidVolume {
                amount: ApproxF32::new(volume_amount, is_approximate),
                unit: unit,
            },
            original_unit: unit_str.map(|s| VolumeUnit::from_str(&s).unwrap()),
        }))
    }

    pub fn parse_value(value: &str) -> (bool, f32) {
        use std::str::FromStr;

        let is_approximate = value.starts_with("~");
        let value = f32::from_str(value.trim_start_matches("~"))
            .expect(&format!("Failed to parse number, '{}'!", value));

        (is_approximate, value)
    }

    pub fn print(&self) -> String {
        let mut display = String::new();

        if self.volume.amount.is_approximate {
            display.push('~');
        }

        display.push_str(&format!("{:.2}", self.volume.amount.num));
        display.push_str(" ");
        display.push_str(self.volume.unit.to_str());

        display
    }
}

#[derive(Clone, Debug)]
pub struct Drink {
    pub name: String,
    pub abv: Option<Abv>,
    pub multiplier: f32,
}

impl Drink {
    pub fn from_entry(entry: &RawEntry) -> Drink {
        let multiplier = entry
            .name
            .as_ref()
            .map(|name| match name.contains("double") {
                true => 2.0,
                false => 1.0,
            })
            .unwrap_or(1.0);

        Drink {
            name: entry
                .name
                .as_ref()
                .expect("Missing drink name!")
                .trim()
                .to_lowercase(),
            abv: Abv::from_entry(entry),
            multiplier: multiplier,
        }
    }
}

impl PartialEq for Drink {
    fn eq(&self, other: &Drink) -> bool {
        self.name == other.name
            && self.abv == other.abv
            && ((self.multiplier * 100.0).trunc() as i32)
                == ((other.multiplier * 100.0).trunc() as i32)
    }
}

impl Eq for Drink {}

impl Hash for Drink {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.abv.hash(state);
        ((self.multiplier * 100.0).trunc() as i32).hash(state);
    }
}

pub struct DrinkSet {
    drinks: HashMap<i32, Drink>,
    lookup: HashMap<Drink, i32>,
}

impl DrinkSet {
    pub fn new() -> DrinkSet {
        DrinkSet {
            drinks: HashMap::new(),
            lookup: HashMap::new(),
        }
    }

    pub fn find(&self, drink: &Drink) -> Option<i32> {
        self.lookup.get(drink).map(|id| *id)
    }

    pub fn insert(&mut self, id: i32, drink: Drink) -> i32 {
        assert!(self.drinks
            .insert(id, drink.clone())
            .is_none(), "Overwrote something!");
        assert!(self.lookup
            .insert(drink, id)
            .is_none(), "Overwrote something!");

        id
    }
}

#[cfg(test)]
mod tests {
    use super::{Abv, QuantityRange, RawEntry};

    #[test]
    fn test_quantity_range_parse_value() {
        assert_eq!((false, 1f32), QuantityRange::parse_value("1"));
        assert_eq!((true, 2f32), QuantityRange::parse_value("~2"));
        assert_eq!((true, 2.1234f32), QuantityRange::parse_value("~2.1234"));
    }

    #[test]
    fn test_quantity_range_parse() {
        let test = |range_tuple, entry_str| {
            assert_eq!(
                make_range(range_tuple),
                QuantityRange::from_entry(&make_quantity_entry(entry_str))
            );
        };
        test((false, 1.0, false, 1.0), "1");
        test((false, 1.0, false, 1.0), "1-1");
        test((true, 1.0, false, 1.0), "~1-1");
        test((true, 1.0, true, 1.0), "~1-~1");
        test((false, 1.0, true, 1.0), "1-~1");

        test((false, 1.5, false, 1.5), "1.5");
        test((true, 2.5, true, 2.5), "~2.5");
        test((false, 66.666, false, 66.666), "66.666");
        test((false, 3.0, false, 5.0), "3-5");
        test((true, 2.0, true, 3.0), "~2-~3");
        test((true, 2.5, true, 3.5), "~2.5-~3.5");

        test((false, 3.0, false, 5.0), "3 - 5");
        test((true, 2.0, true, 3.0), "~2 - ~3");
        test((true, 2.5, true, 3.5), "~2.5 - ~3.5");
        test((false, 1.0, false, 2.0), "1-2");
    }

    #[test]
    fn test_abv_parse() {
        let test = |abv_tuple, entry_str| {
            assert_eq!(
                make_abv(abv_tuple),
                Abv::from_entry(&make_abv_entry(entry_str)).unwrap()
            );
        };
        test((false, 1.0, false, 1.0), "1%");
        test((false, 1.0, false, 1.0), "1-1%");
        test((true, 1.0, false, 1.0), "~1-1%");
        test((true, 1.0, true, 1.0), "~1-~1%");
        test((false, 1.0, true, 1.0), "1-~1%");

        test((false, 1.0, false, 1.0), "1%");
        test((false, 1.0, false, 1.0), "1%-1%");
        test((true, 1.0, false, 1.0), "~1%-1%");
        test((true, 1.0, true, 1.0), "~1%-~1%");
        test((false, 1.0, true, 1.0), "1%-~1%");

        test((false, 1.5, false, 1.5), "1.5%");
        test((true, 2.5, true, 2.5), "~2.5%");
        test((false, 66.666, false, 66.666), "66.666%");
        test((false, 3.0, false, 5.0), "3-5%");
        test((false, 3.0, false, 5.0), "3%-5%");
        test((true, 2.0, true, 3.0), "~2-~3%");
        test((true, 2.0, true, 3.0), "~2%-~3%");
        test((true, 2.5, true, 3.5), "~2.5-~3.5%");
        test((true, 2.5, true, 3.5), "~2.5%-~3.5%");

        test((false, 3.0, false, 5.0), "3 - 5%");
        test((false, 3.0, false, 5.0), "3% - 5%");
        test((true, 2.0, true, 3.0), "~2 - ~3%");
        test((true, 2.0, true, 3.0), "~2% - ~3%");
        test((true, 2.5, true, 3.5), "~2.5 - ~3.5%");
        test((true, 2.5, true, 3.5), "~2.5% - ~3.5%");
        test((false, 1.0, false, 2.0), "1-2%");
        test((false, 1.0, false, 2.0), "1%-2%");
    }

    fn make_quantity_entry(quantity: &str) -> RawEntry {
        RawEntry {
            date: None,
            quantity: Some(quantity.into()),
            name: None,
            abv: None,
            volume: None,
        }
    }

    fn make_abv_entry(abv: &str) -> RawEntry {
        RawEntry {
            date: None,
            quantity: None,
            name: None,
            abv: Some(abv.into()),
            volume: None,
        }
    }

    fn make_range(tuple: (bool, f32, bool, f32)) -> QuantityRange {
        let (apprx_min, min, apprx_max, max) = tuple;

        QuantityRange {
            min,
            max,
            approximate_min: apprx_min,
            approximate_max: apprx_max,
        }
    }

    fn make_abv(tuple: (bool, f32, bool, f32)) -> Abv {
        let (apprx_min, min, apprx_max, max) = tuple;

        Abv {
            min,
            max,
            approximate_min: apprx_min,
            approximate_max: apprx_max,
        }
    }
}
