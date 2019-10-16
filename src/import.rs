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
pub struct Date {
    pub day: String,
    pub time: String,
    pub context: Vec<String>,
}

impl Date {
    pub fn from_entry(entry: &RawEntry, previous: &Date) -> Date {
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

        let day = cap_str("day").unwrap_or(previous.day.clone());
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
                false => match day == previous.day {
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

        Date {
            day: day,
            time: time,
            context: context,
        }
    }
}
