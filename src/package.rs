// Validates and parses a zip file.
use crate::{
    description::to_html,
    manifest::{load_package_manifest, PackageManifest},
};

use actix_web::web::Bytes;
use serde::Serialize;
use tokio::{
    task::spawn_blocking,
    time::{error::Elapsed, timeout, Duration},
};
use toml::de::Error;
use zip::{read::ZipFile, result::ZipError, ZipArchive};

use std::io::{Cursor, Error as IOError, Read, Seek};

#[derive(Debug, Serialize)]
pub struct Package {
    pub manifest: PackageManifest,
    pub description: String,
}

#[derive(Debug)]
pub enum ParseError {
    ZipError(ZipError),
    TOMLError(Error),
    Timeout,
}

pub type ParseResult<T> = Result<T, ParseError>;

impl From<ZipError> for ParseError {
    fn from(e: ZipError) -> Self {
        Self::ZipError(e)
    }
}

impl From<IOError> for ParseError {
    fn from(e: IOError) -> Self {
        Self::ZipError(ZipError::Io(e))
    }
}

impl From<Error> for ParseError {
    fn from(e: Error) -> Self {
        Self::TOMLError(e)
    }
}

impl From<Elapsed> for ParseError {
    fn from(_: Elapsed) -> Self {
        Self::Timeout
    }
}

fn read_file(mut file: ZipFile) -> ParseResult<String> {
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn parse(reader: impl Read + Seek) -> ParseResult<Package> {
    let mut zip = ZipArchive::new(reader)?;

    let readme_md = {
        let readme = zip.by_name("README.md")?;
        read_file(readme)?
    };

    let manifest_str = {
        let manifest = zip.by_name("aopkg.toml")?;
        read_file(manifest)?
    };

    let manifest = load_package_manifest(&manifest_str)?;
    let description = to_html(&readme_md);

    Ok(Package {
        manifest,
        description,
    })
}

pub async fn try_parse(reader: Cursor<Bytes>) -> ParseResult<Package> {
    let task = spawn_blocking(move || parse(reader));
    timeout(Duration::from_secs(5), task).await?.unwrap()
}
