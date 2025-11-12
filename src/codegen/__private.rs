use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use byteorder::{BigEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Blob {
    pub data: Vec<u8>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("option {0} is missing")]
    MissingOption(String),

    #[error("bad value \"{0}\" given for {1}")]
    BadValue(String, String),

    #[error("unexpected data in file {0}: {1}")]
    UnexpectedData(String, String),

    #[error("io error {0}")]
    IO(std::io::Error),
}

pub enum FromBytesError {
    UnexpectedData(String),
}

fn read_from_fs<T>(filename: impl Into<PathBuf>) -> Result<T, Error>
where
    T: FromBytes,
{
    let filepath = filename.into();
    let mut file = File::open(filepath.clone()).map_err(Error::IO)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(Error::IO)?;

    match T::from_bytes(buffer) {
        Ok(data) => Ok(data),
        Err(e) => match e {
            FromBytesError::UnexpectedData(msg) => Err(Error::UnexpectedData(
                filepath.to_string_lossy().to_string(),
                msg,
            )),
        },
    }
}

pub fn get_option<T>(name: &str, fs: bool) -> Result<T, Error>
where
    T: FromStr + FromBytes,
{
    let value = std::env::var(name).map_err(|_| Error::MissingOption(name.to_string()))?;
    if fs {
        read_from_fs(value)
    } else {
        Ok(value
            .parse::<T>()
            .map_err(|_| Error::BadValue(value, name.to_string()))?)
    }
}

pub fn get_options<T>(name: &str, fs: bool) -> Result<Vec<T>, Error>
where
    T: FromStr + FromBytes,
{
    let separator = if fs {
        if cfg!(windows) { ';' } else { ':' }
    } else {
        ','
    };

    let value = std::env::var(name).map_err(|_| Error::MissingOption(name.to_string()))?;
    if fs {
        value.split(separator).map(|x| read_from_fs(x)).collect()
    } else {
        value
            .split(separator)
            .map(|x| T::from_str(x).map_err(|_| Error::BadValue(name.to_string(), x.to_string())))
            .collect()
    }
}

pub trait FromBytes {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError>
    where
        Self: Sized;
}

impl FromBytes for u8 {
    fn from_bytes(mut x: Vec<u8>) -> Result<Self, FromBytesError> {
        x.pop()
            .ok_or(FromBytesError::UnexpectedData("file is empty".to_string()))
    }
}

impl FromBytes for i8 {
    fn from_bytes(mut x: Vec<u8>) -> Result<Self, FromBytesError> {
        x.pop()
            .ok_or(FromBytesError::UnexpectedData("file is empty".to_string()))
            .map(|x| x as i8)
    }
}

impl FromBytes for u16 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_u16::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for i16 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_i16::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for u32 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_u32::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for i32 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_i32::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for u64 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_u64::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for i64 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_i64::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for f32 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_f32::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for f64 {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Cursor::new(x)
            .read_f64::<BigEndian>()
            .map_err(|_| FromBytesError::UnexpectedData("not enough bytes".to_string()))
    }
}

impl FromBytes for String {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        String::from_utf8(x)
            .map_err(|_| FromBytesError::UnexpectedData("invalid utf8 bytes".to_string()))
    }
}

impl FromBytes for bool {
    fn from_bytes(mut x: Vec<u8>) -> Result<Self, FromBytesError> {
        x.pop()
            .ok_or(FromBytesError::UnexpectedData("file is empty".to_string()))
            .map(|x| x != 0)
    }
}

impl FromBytes for Blob {
    fn from_bytes(x: Vec<u8>) -> Result<Self, FromBytesError> {
        Ok(Blob { data: x })
    }
}

impl FromStr for Blob {
    type Err = FromBytesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Blob {
            data: BASE64_STANDARD
                .decode(s)
                .map_err(|_| FromBytesError::UnexpectedData("invalid base64".to_string()))?,
        })
    }
}
