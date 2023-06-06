use crate::error::{Error, Result};
use colored::Colorize;
use std::{fmt, fs, io, path};
use terminal_size::{terminal_size, Width};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Kind {
    Directory,
    Symlink,
    File,
}

impl From<fs::FileType> for Kind {
    fn from(file_type: fs::FileType) -> Self {
        if file_type.is_file() {
            Kind::File
        } else if file_type.is_dir() {
            Kind::Directory
        } else {
            Kind::Symlink
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Entry {
    pub kind: Kind,
    pub name: String,
}

impl TryFrom<fs::DirEntry> for Entry {
    type Error = Error;
    fn try_from(dir_entry: fs::DirEntry) -> Result<Self> {
        Ok(Self {
            name: dir_entry.file_name().into_string()?,
            kind: dir_entry.file_type()?.into(),
        })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Directory => write!(f, "{}", self.name.blue().bold()),
            Kind::Symlink => write!(f, "{}", self.name.bright_blue().underline()),
            Kind::File => write!(f, "{}", self.name),
        }
    }
}

pub fn read_entries(path: path::PathBuf, all: bool) -> Result<Vec<Entry>> {
    let dir_entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    let mut entries = Vec::new();

    for dir_entry in dir_entries {
        let entry: Entry = dir_entry.try_into()?;
        if !all && entry.name.starts_with('.') {
            continue;
        }

        entries.push(entry);
    }

    Ok(entries)
}

pub fn print_entries(entries: Vec<Entry>, long: bool) {
    if long {
        for entry in entries {
            println!("{entry}");
        }
        return;
    }

    let Width(width) = terminal_size().map_or(Width(0), |size| size.0);

    let entries = entries
        .into_iter()
        .map(|entry| {
            let entry_width = match entry.kind {
                Kind::Directory => entry.name.len() + 1,
                _ => entry.name.len(),
            };
            (entry, entry_width)
        })
        .collect::<Vec<_>>();

    let max_entry_width = entries.iter().map(|entry| entry.1).max().unwrap_or(0);

    let entries_per_line = (width as usize) / (max_entry_width + 2);
    let mut entry_idx: usize = 0;

    for (entry, entry_width) in entries {
        entry_idx += 1;
        if entry_idx >= entries_per_line {
            entry_idx = 0;
            println!("{entry}");
        } else {
            print!("{entry}{}  ", " ".repeat(max_entry_width - entry_width));
        }
    }

    // last print wasn't a `println`, add final newline
    if entry_idx != 0 {
        println!();
    }
}
