use chrono::prelude::*;
use regex::Regex;

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
    pub time: String,
    pub context: Vec<String>,
}

impl DateContext {
    pub fn from_entry(entry: &RawEntry, previous: &DateContext) -> DateContext {
        lazy_static! {
            static ref RE: Regex = Regex::new(
                r#"^(?P<day>(?:\d{1,2}\s\w{3})|(?:\w{3}\s\d{1,2}))?[,; ]*(?:(?P<context2>[^\r\n;,]*?)[;,]?)?(?:(?P<context1>[^\r\n;,]*?)[;,]?)?$"#
            )
            .unwrap();

            static ref TIMES: Vec<&'static str> = vec!["morning", "afternoon", "evening", "night"];
            static ref BRUNCH: String = String::from("brunch");
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
                .map(|c| TIMES.contains(&c.as_ref()))
                .unwrap_or(false)
        };

        // I frequently just write "brunch"; if so we'll mark this as "afternoon".
        let is_brunch =
            context1.contains(&BRUNCH as &String) || context2.contains(&BRUNCH as &String);

        let time = match (
            is_time_string(context1.as_ref()),
            is_time_string(context2.as_ref()),
        ) {
            // If one of either is a time specifier, then use that value.
            (true, false) => context1.clone().unwrap(),
            (false, true) => context2.clone().unwrap(),
            // If neither specify the time perioud, first check if "brunch" was present.
            (false, false) => match is_brunch {
                // If it was, then use "afternoon"
                true => String::from("afternoon"),
                // Otherwise, if this record is the same day as the previous,
                // then continue using the same time as the previous.
                // Use "night" otherwise.
                false => match date == previous.date {
                    true => previous.time.clone(),
                    false => String::from("night"),
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
            .filter(|c| c.is_some() && !c.contains(&time))
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
}

#[derive(PartialEq, Debug)]
pub struct QuantityRange {
    min: f32,
    max: f32,
    approximate_min: bool,
    approximate_max: bool,
}

impl QuantityRange {
    pub fn from_entry(entry: &RawEntry) -> QuantityRange {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"(~?\d+(?:\.\d+)?)(?:\s*\-\s*(~?\d+(?:\.\d+)?))?"#).unwrap();
        }

        let captures = RE
            .captures(&entry.quantity.as_ref().expect("No quantity found!"))
            .unwrap();

        let cap_index = |index| {
            captures
                .get(index)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
        };

        let min = cap_index(1)
            .map(Self::parse_value)
            .expect("A minimum quantity is required!");
        let max = cap_index(2).map(Self::parse_value).unwrap_or(min);

        QuantityRange {
            min: min.1,
            max: max.1,
            approximate_min: min.0,
            approximate_max: max.0,
        }
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

        if self.approximate_min {
            display.push_str("~");
        }

        display.push_str(&format!("{:.2}", self.min));

        if self.min != self.max || self.approximate_min != self.approximate_max {
            display.push('-');

            if self.approximate_max {
                display.push_str("~");
            }

            display.push_str(&format!("{:.2}", self.max));
        }

        display
    }
}

#[derive(PartialEq, Debug)]
pub struct Abv {
    min: f32,
    max: f32,
    approximate_min: bool,
    approximate_max: bool,
}

impl Abv {
    pub fn from_entry(entry: &RawEntry) -> Option<Abv> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"(~?\d+(?:\.\d+)?)%?(?:\s*\-\s*(~?\d+(?:\.\d+)?)%?)?%"#).unwrap();
        }

        if entry.abv.is_none() {
            return None;
        }

        let captures = match RE.captures(&entry.abv.as_ref().expect("No ABV found!")) {
            Some(c) => c,
            None => return None,
        };

        let cap_index = |index| {
            captures
                .get(index)
                .map(|m| m.as_str().trim())
                .filter(|s| *s != "")
        };

        let min = cap_index(1)
            .map(Self::parse_value)
            .expect("A minimum ABV is required!");
        let max = cap_index(2).map(Self::parse_value).unwrap_or(min);

        Some(Abv {
            min: min.1,
            max: max.1,
            approximate_min: min.0,
            approximate_max: max.0,
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

        if self.approximate_min {
            display.push_str("~");
        }

        display.push_str(&format!("{:.1}", self.min));

        if self.min != self.max || self.approximate_min != self.approximate_max {
            display.push('-');

            if self.approximate_max {
                display.push_str("~");
            }

            display.push_str(&format!("{:.1}", self.max));
        }

        display.push('%');

        display
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
