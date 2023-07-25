use argh::FromArgs;
use const_format::concatcp;
use filetime::FileTime;
use serde_derive::{Deserialize, Serialize};
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::time::Duration;

#[derive(FromArgs)]
/// Retrieve the verse-of-the-day or a specified verse from NET Bible. Verses
/// are case-insensitive, and some short names are acceptable (based on the NET
/// Bible API, not the CLI). "random" and "votd" are accepted verses, and do
/// what they sound like.
struct VerseOpts {
    /// disable reading from/writing to the cache (only affects VotD)
    #[argh(switch, short = 'n')]
    no_cache: bool,

    /// get the current VotD from the web (not cache), then write it to cache
    #[argh(switch, short = 'r')]
    refresh_cache: bool,

    /// only display the text of the verse(s), with no title before
    #[argh(switch, short = 'o')]
    only_verse: bool,

    /// print the translation (NET) after the verse name
    #[argh(switch)]
    show_translation: bool,

    /// specify a timeout to quit the request after (in seconds); defaults to 2
    #[argh(option, default = "2", short = 't')]
    timeout: u64,

    #[argh(positional, greedy)]
    verse: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Verse {
    title: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiVerse {
    bookname: String,
    chapter: String,
    verse: String,
    text: String,
}

const VERSE_URL: &str = "https://labs.bible.org/api/?type=json";
const URL_PARSE_ERROR: &str = concatcp!(VERSE_URL, " should be a valid URL");
const CACHE_EXPIRE_TIME: i64 = 21600; // 1/4 a day, in seconds

fn cache_file_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.cache_dir().join("votd-cli-cache.txt"))
}

async fn fetch_verse(verse: Option<&str>, timeout: Duration) -> reqwest::Result<Verse> {
    let url = reqwest::Url::parse_with_params(
        VERSE_URL,
        &[("passage", if let Some(s) = verse { s } else { "votd" })],
    )
    .expect(URL_PARSE_ERROR);
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    // The API returns status code 400 and a blank page when given an invalid
    // verse to look-up. To work around this, `error_for_status()` is used for
    // an early return instead of trying to parse an empty page as JSON.
    let verses = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<ApiVerse>>()
        .await?;
    assert!(!verses.is_empty(), "No verses returned");
    let book = &verses[0].bookname;
    let chapter = verses[0]
        .chapter
        .parse::<i32>()
        .expect("Chapters should be valid integers");
    let verse_start = verses[0]
        .verse
        .parse::<i32>()
        .expect("Verses should be valid integers");
    let verse_end = verses[verses.len() - 1]
        .verse
        .parse::<i32>()
        .expect("Verses should be valid integers");
    Ok(Verse {
        title: if verse_start == verse_end {
            format!("{} {}:{}", book, chapter, verse_start)
        } else {
            format!("{} {}:{}-{}", book, chapter, verse_start, verse_end)
        },
        text: verses.iter().fold("".to_owned(), |acc, e| acc + &e.text),
    })
}

fn unwrap_error<T>(res: reqwest::Result<T>) -> T {
    match res {
        Ok(x) => x,
        Err(e) => {
            if e.is_timeout() {
                eprintln!("Error: timeout exceeded");
            } else if e.is_status() {
                eprintln!("Server returned an error; is the verse you requested valid?");
            } else if e.is_connect() {
                eprintln!("Couldn't connect to server; are you connected to the Internet?");
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let args: VerseOpts = argh::from_env();
    let verse_requested = if !args.verse.is_empty() {
        Some(args.verse.join(" "))
    } else {
        None
    };

    let timeout = Duration::from_secs(args.timeout);

    let mut cache = if verse_requested.is_none() && !args.no_cache {
        if let Some(path) = cache_file_path() {
            let mut cache_file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(false)
                .open(path)
                .unwrap();
            cache_file.rewind().unwrap();
            let metadata = cache_file.metadata().unwrap();
            let stamp = FileTime::from_last_modification_time(&metadata).seconds();
            let now = FileTime::now().seconds();
            Some((cache_file, now - stamp <= CACHE_EXPIRE_TIME && !args.refresh_cache))
        } else {
            println!("Can't determine where to place a cache file. Skipping.");
            None
        }
    } else {
        None
    };

    let (verse, write_cache) = if let Some((cache_file, true)) = cache.as_mut() {
        let mut buf = Vec::new();
        cache_file.read_to_end(&mut buf).unwrap();
        let res = rmp_serde::from_slice(&buf);
        cache_file.rewind().unwrap();
        if let Ok(cached) = res {
            (cached, false)
        } else {
            // for `cache` to be `Some`, `verse_requested` must be `None` and
            // `no_cache` must be `false`, so we can write to cache
            (unwrap_error(fetch_verse(None, timeout).await), true)
        }
    } else {
        let verse = unwrap_error(fetch_verse(verse_requested.as_deref(), timeout).await);
        (verse, verse_requested.is_none() && !args.no_cache)
    };

    if !args.only_verse {
        print!("{}", verse.title);
        if args.show_translation {
            print!(
                " ({})",
                if verse_requested.is_none() {
                    "Verse of the Day - NET"
                } else {
                    "NET"
                }
            );
        } else if verse_requested.is_none() {
            print!(" (Verse of the Day)");
        }
        println!();
    }
    let options = textwrap::Options::with_termwidth();
    let wrapped = textwrap::wrap(&verse.text, &options);
    for line in wrapped {
        println!("{}", line);
    }

    if write_cache && cache.is_some() {
        let (mut cache_file, _) = cache.expect("Cache has to contain a value to reach this code");
        rmp_serde::encode::write(&mut cache_file, &verse).unwrap();
    }
}
