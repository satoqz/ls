use crate::error::{Error, Result};
use colored::Colorize;
use std::{
    cmp, ffi, fmt, fs, io,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path,
};
use terminal_size::{terminal_size, Width};

macro_rules! max_field_width {
    ($vec:expr, $field:ident) => {
        $vec.iter().map(|v| v.$field.len()).max().unwrap_or(0)
    };
}

const KILOBYTE: u64 = 1000;
const GIGABYTE: u64 = KILOBYTE * 1000;
const TERABYTE: u64 = GIGABYTE * 1000;

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

#[derive(Debug)]
pub struct Entry {
    kind: Kind,
    name: String,
    metadata: fs::Metadata,
    dir_entry: fs::DirEntry,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.kind == other.kind && self.name == other.name
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Entry) -> Option<cmp::Ordering> {
        Some(match self.kind.cmp(&other.kind) {
            cmp::Ordering::Equal => {
                match self.name.to_lowercase().cmp(&other.name.to_lowercase()) {
                    cmp::Ordering::Equal => self.name.cmp(&other.name),
                    ordering => ordering,
                }
            }
            ordering => ordering,
        })
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Entry) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl TryFrom<fs::DirEntry> for Entry {
    type Error = Error;

    fn try_from(dir_entry: fs::DirEntry) -> Result<Self> {
        Ok(Self {
            name: dir_entry.file_name().into_string()?,
            kind: dir_entry.file_type()?.into(),
            metadata: dir_entry.metadata()?,
            dir_entry,
        })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::Directory => write!(f, "{}/", self.name.blue().bold()),
            Kind::Symlink => write!(f, "{}", self.name.bright_blue().underline()),
            Kind::File if self.is_executable() => write!(f, "{}", self.name.bright_green()),
            Kind::File => write!(f, "{}", self.name),
        }
    }
}

impl Entry {
    fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    fn is_executable(&self) -> bool {
        if self.kind != Kind::File {
            return false;
        }

        self.metadata.permissions().mode() & 0o111 != 0
    }

    fn len(&self) -> usize {
        if self.kind == Kind::Directory {
            self.name.len() + 1
        } else {
            self.name.len()
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct LongEntry {
    entry: Entry,
    owner: String,
    group: String,
    size: String,
    link_target: Option<String>,
}

impl TryFrom<Entry> for LongEntry {
    type Error = Error;

    fn try_from(entry: Entry) -> Result<Self> {
        let owner = unsafe {
            let passwd = libc::getpwuid(entry.metadata.uid());
            ffi::CStr::from_ptr((*passwd).pw_name).to_string_lossy()
        }
        .to_string();

        let group = unsafe {
            let group = libc::getgrgid(entry.metadata.gid());
            ffi::CStr::from_ptr((*group).gr_name).to_string_lossy()
        }
        .to_string();

        let size = match entry.metadata.size() {
            b if b < KILOBYTE => format!("{b}B"),
            b if b < GIGABYTE => format!("{}.{}K", b / KILOBYTE, (b % KILOBYTE) / (KILOBYTE / 10)),
            b if b < TERABYTE => format!("{}.{}G", b / GIGABYTE, (b % GIGABYTE) / (GIGABYTE / 10)),
            b => format!("{}.{}T", b / TERABYTE, (b % TERABYTE) / (TERABYTE / 10)),
        };

        let link_target = if entry.kind == Kind::Symlink {
            let target = fs::read_link(entry.dir_entry.path())?;
            Some(target.to_string_lossy().into_owned())
        } else {
            None
        };

        Ok(Self {
            entry,
            owner,
            group,
            size,
            link_target,
        })
    }
}

impl fmt::Display for LongEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.entry.kind == Kind::Symlink {
            return write!(
                f,
                "{} -> {}",
                self.entry.name.bright_blue().underline(),
                self.link_target.clone().unwrap()
            );
        }

        self.entry.fmt(f)
    }
}

pub fn read_entries(path: path::PathBuf, all: bool) -> Result<Vec<Entry>> {
    let dir_entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    let mut entries = Vec::new();

    for dir_entry in dir_entries {
        let entry: Entry = dir_entry.try_into()?;
        if !all && entry.is_hidden() {
            continue;
        }

        entries.push(entry);
    }

    Ok(entries)
}

pub fn print_entries_short(entries: Vec<Entry>) {
    let Width(width) = terminal_size().map_or(Width(80), |size| size.0);

    // everything can fit in one line, don't bother with creating an aligned table
    if width as usize >= entries.iter().fold(0, |acc, entry| acc + entry.len() + 2) - 2 {
        for (index, entry) in entries.iter().enumerate() {
            if index == entries.len() - 1 {
                println!("{entry}");
            } else {
                print!("{entry}  ");
            }
        }

        return;
    }

    let max_entry_width = entries.iter().map(Entry::len).max().unwrap_or(0);
    let entries_per_line = (width as usize) / (max_entry_width + 2);

    for (index, entry) in entries.iter().enumerate() {
        if (index + 1) % entries_per_line == 0 {
            println!("{entry}");
        } else {
            print!("{entry}{}  ", " ".repeat(max_entry_width - entry.len()));
        }
    }

    // last print wasn't a `println`, add final newline
    if entries.len() % entries_per_line != 0 {
        println!();
    }
}

pub fn print_entries_long(entries: Vec<LongEntry>) {
    let max_owner_width = max_field_width!(entries, owner);
    let max_group_width = max_field_width!(entries, group);
    let max_size_width = max_field_width!(entries, size);

    for entry in entries {
        println!(
            "{}{} {}{} {}{} {}",
            " ".repeat(max_owner_width - entry.owner.len()),
            entry.owner,
            " ".repeat(max_group_width - entry.group.len()),
            entry.group,
            " ".repeat(max_size_width - entry.size.len()),
            entry.size,
            entry
        );
    }
}
