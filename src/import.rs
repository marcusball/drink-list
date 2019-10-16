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
