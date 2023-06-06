mod entry;
mod error;

use crate::error::{Error, Result};
use std::{env, path};

fn main() {
    let (all, long) = parse_flags();

    let cwd = env::current_dir()
        .map_err(Error::from)
        .unwrap_or_else(|err| err.print_and_exit());

    let path = parse_args().map(|path| cwd.join(path)).unwrap_or(cwd);

    let mut entries = entry::read_entries(path, all).unwrap_or_else(|err| err.print_and_exit());
    entries.sort();

    if long {
        let long_entries: Vec<entry::LongEntry> = entries
            .into_iter()
            .map(entry::Entry::try_into)
            .collect::<Result<Vec<_>>>()
            .unwrap_or_else(|err| err.print_and_exit());
        entry::print_entries_long(long_entries);
    } else {
        entry::print_entries_short(entries);
    }
}

fn parse_flags() -> (bool, bool) {
    let mut all = false;
    let mut long = false;

    let args = env::args()
        .skip(1)
        .take_while(|arg| arg.starts_with('-') && arg != "--");

    for arg in args {
        for flag in arg
            .trim_start_matches('-')
            .split("")
            .filter(|flag| !flag.is_empty())
        {
            if flag == "a" {
                all = true;
            } else if flag == "l" {
                long = true;
            } else {
                Error::UnknownFlag(flag.into()).print_and_exit();
            }
        }
    }

    (all, long)
}

fn parse_args() -> Option<path::PathBuf> {
    env::args()
        .skip(1)
        .skip_while(|arg| arg.starts_with('-') && arg != "--")
        .find(|arg| arg != "--")
        .map(path::PathBuf::from)
}
