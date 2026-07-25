use argh::FromArgs;
use std::time::Duration;

mod api;
use api::{Formatting, fetch_verse};
mod cache;
use cache::Cache;

#[derive(FromArgs)]
/// Retrieve the verse-of-the-day or a specified passage from NET Bible. Verses
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

    /// display the program version and exit
    #[argh(switch, short = 'v')]
    version: bool,

    /// don't wrap the text of the verse(s) to the terminal width
    //
    // NOTE: on later design I'm not a huge fan of this, it sorta violates the
    // UNIX philosophy, and is easy to implement via the POSIX tool `fold`.
    //
    // I'm not removing it now, to avoid a backwards-incompatible change, but
    // I'm flagging this for potential removal later.
    #[argh(switch, short = 'w')]
    no_wrap: bool,

    /// add a divider between separate passages
    #[argh(switch, short = 'd')]
    divider: bool,

    /// specify the text formatting, out of `full`, `para`, `bold`, and `plain`
    #[argh(option, default = "Formatting::Plain", short = 'f')]
    formatting: Formatting,

    #[argh(positional)]
    verse: Vec<String>,
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

fn main() {
    let args: VerseOpts = argh::from_env();
    if args.version {
        println!("VotD v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let verse_requested = if !args.verse.is_empty() {
        args.verse.join(" ")
    } else {
        String::from("votd")
    };

    let timeout = Duration::from_secs(args.timeout);

    // only cache the verse of the day
    let mut cache = if verse_requested == "votd" && !args.no_cache {
        match Cache::new() {
            Ok(c) => Some(c),
            Err(err) => {
                eprintln!("Error opening cache: {}", err);
                None
            }
        }
    } else {
        None
    };

    let verses = if let Some(cache) = cache.as_mut() && cache.is_live() && !args.refresh_cache {
        cache.read().or_else(|err| {
            eprintln!("Error reading cache: {}", err);
            fetch_verse(&verse_requested, args.formatting, timeout)
        })
    } else {
        fetch_verse(&verse_requested, args.formatting, timeout)
    };
    let verses = unwrap_error(verses);

    let size = terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .filter(|_| !args.no_wrap);

    for v in &verses {
        if v != verses.first().expect("`verses` is known to be non-empty") && args.divider {
            println!();
            println!("------------------------------");
            println!();
        }

        if !args.only_verse {
            print!("{}", v.title);
            if args.show_translation {
                print!(
                    " ({})",
                    if verse_requested == "votd" {
                        "Verse of the Day - NET"
                    } else {
                        "NET"
                    }
                );
            } else if verse_requested == "votd" {
                print!(" (Verse of the Day)");
            }
            println!();
        }

        if let Some(size) = size {
            let wrapped = textwrap::wrap(&v.text, size);
            for line in wrapped {
                println!("{}", line);
            }
        } else {
            println!("{}", v.text);
        }
    }

    if let Some(cache) = cache.as_mut() && !args.no_cache {
        cache.write(&verses).unwrap();
    }
}
