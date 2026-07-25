## 1.1.0

Added better support for requesting multiple passages at the same time, e.g. `votd Eph 4 \; Matt 5` (the semi-colon is optional).

Added the `--formatting` flag (`-f` for short) and set the default to plain formatting, instead of having `<b>` tags. (Those can be reintroduced with `-f bold`)

Cleaned up the code base some, separating out some files from the `main.rs`
