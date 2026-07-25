use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Formatting {
    Full,
    Para,
    Bold,
    Plain,
}

impl Formatting {
    pub fn str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Para => "para",
            Self::Bold => "",
            Self::Plain => "plain",
        }
    }
}

impl std::str::FromStr for Formatting {
    type Err = String;

    fn from_str(s: &str) -> Result<Formatting, String> {
        match &*s.to_lowercase() {
            "full" => Ok(Self::Full),
            "para" => Ok(Self::Para),
            "bold" => Ok(Self::Bold),
            "plain" => Ok(Self::Plain),
            _ => Err(format!("`{}' is not a valid formatting option", s)),
        }
    }
}

impl fmt::Display for Formatting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.str())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct ApiVerse {
    bookname: String,
    chapter: String,
    verse: String,
    text: String,
}

const VERSE_URL: &str = "https://labs.bible.org/api/?type=json";

fn parse_body(json: Vec<ApiVerse>) -> Verse {
    let book = &json[0].bookname;
    let chapter = json[0]
        .chapter
        .parse::<i32>()
        .expect("Chapters should be valid integers");
    let verse_start = json[0]
        .verse
        .parse::<i32>()
        .expect("Verses should be valid integers");
    let verse_end = json[json.len() - 1]
        .verse
        .parse::<i32>()
        .expect("Verses should be valid integers");
    Verse {
        title: if verse_start == verse_end {
            format!("{} {}:{}", book, chapter, verse_start)
        } else {
            format!("{} {}:{}-{}", book, chapter, verse_start, verse_end)
        },
        text: json.iter().fold("".to_owned(), |acc, e| acc + &e.text),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Verse {
    pub title: String,
    pub text: String,
}

pub fn fetch_verse(
    verse: &str,
    formatting: Formatting,
    timeout: Duration,
) -> reqwest::Result<Vec<Verse>> {
    let url = reqwest::Url::parse_with_params(
        VERSE_URL,
        &[("passage", verse), ("formatting", formatting.str())],
    )
    .expect("Bible API URL should be a valid URL");
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    // The API returns status code 400 and a blank page when given an invalid
    // verse to look-up. To work around this, `error_for_status()` is used for
    // an early return instead of trying to parse an empty page as JSON.
    let verses = client
        .get(url)
        .header(
            reqwest::header::HeaderName::from_static("user-agent"),
            "Mozilla/5.0 Gecko/20100101 Firefox/130.0",
        )
        .send()?
        .error_for_status()?
        .json::<Vec<ApiVerse>>()?;
    assert!(!verses.is_empty(), "No verses returned");

    let mut out = Vec::new();
    for (_, chunk) in &verses.iter().chunk_by(|elt| (&elt.bookname, &elt.chapter)) {
        out.push(parse_body(chunk.cloned().collect()));
    }
    Ok(out)
}
