# VotD

[![Crates.io](https://img.shields.io/crates/v/votd.svg)](https://crates.io/crates/votd)
[![MIT License](https://img.shields.io/github/license/MWPuppire/votd.js.svg)](https://github.com/MWPuppire/votd/blob/master/LICENSE)

A command-line utility to look up the Bible verse-of-the-day. Use `votd` to get the current verse-of-the-day, or `votd --help` for command-line flags.

You can install it via cargo:
```
$ cargo install votd
```

## Maintenance

I consider this a finished program; it serves my needs, and I don't care to work more on it. I may address significant issues (e.g. major bugs, vulnerabilities, or if the API routes change), but if you want smaller changes made, feel free to make a fork.

## Performance

It's written in Rust for performance; I include it in my `.zshrc` file, so I want it to be pretty fast. It caches the verse-of-the-day for 6 hours; this should mean it always returns a current one (most people need at least that much sleep between days), but the cache can be disabled with `-n` if it isn't desired.

To avoid using too much filesystem space, it doesn't cache any verses other than the verse-of-the-day. If you want an app to look-up local copies of any verse, you'd be better off downloading an actual Bible app anyways.

## License

The code is released under the MIT license.
