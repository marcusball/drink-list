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
