use colored::Colorize;
use std::{ffi, fmt, io, process, result};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    StringConversion,
    UnknownFlag(String),
}

pub type Result<T> = result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(io_error: io::Error) -> Self {
        Self::Io(io_error)
    }
}

impl From<ffi::OsString> for Error {
    fn from(_: ffi::OsString) -> Self {
        Self::StringConversion
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", "Error".red().bold(), self.message())
    }
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Error::Io(io_err) => io_err.to_string(),
            Error::StringConversion => "File name includes invalid Unicode data".into(),
            Error::UnknownFlag(flag) => format!("Unknown flag \"{flag}\""),
        }
    }

    pub fn code(&self) -> i32 {
        match self {
            Error::Io(io_err) => io_err.raw_os_error().unwrap_or(1),
            _ => 1,
        }
    }

    pub fn print_and_exit(&self) -> ! {
        eprintln!("{self}");
        process::exit(self.code());
    }
}
