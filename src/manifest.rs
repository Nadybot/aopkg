// Parser for the nadypkg.toml files
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sqlx::{ColumnIndex, Decode, Error as SqlxError, FromRow, Row, Type};
use toml::{de::Error, from_str};

use std::{
    convert::TryFrom,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub enum BotType {
    Nadybot,
    Tyrbot,
    Budabot,
    BeBot,
}

impl Display for BotType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Nadybot => write!(f, "Nadybot"),
            Self::Tyrbot => write!(f, "Tyrbot"),
            Self::BeBot => write!(f, "BeBot"),
            Self::Budabot => write!(f, "Budabot"),
        }
    }
}

impl TryFrom<String> for BotType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Nadybot" => Ok(Self::Nadybot),
            "Tyrbot" => Ok(Self::Tyrbot),
            "Budabot" => Ok(Self::Budabot),
            "BeBot" | "Bebot" => Ok(Self::BeBot),
            _ => Err("Unknown bot type"),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct PackageManifest {
    pub name: String,
    pub description: String,
    pub version: Version,
    pub author: String,
    pub bot_type: BotType,
    pub bot_version: VersionReq,
    pub github: Option<String>,
}

#[derive(Serialize)]
pub struct PackageManifestDb {
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub version: Version,
    pub author: String,
    pub owner: i64,
    pub bot_type: BotType,
    pub bot_version: VersionReq,
    pub github: Option<String>,
}

#[derive(FromRow, Decode)]
pub struct PackageDb {
    pub id: i64,
    pub owner: i64,
}

impl<'r, 's, R> FromRow<'r, R> for PackageManifestDb
where
    R: Row,
    &'s str: ColumnIndex<R>,
    String: Type<R::Database> + Decode<'r, R::Database>,
    i64: Type<R::Database> + Decode<'r, R::Database>,
{
    #[inline]
    fn from_row(row: &'r R) -> Result<Self, SqlxError> {
        let name: String = row.try_get("name")?;
        let description: String = row.try_get("description")?;
        let short_description: String = row.try_get("short_description")?;
        let version: String = row.try_get("version")?;
        let author: String = row.try_get("author")?;
        let bot_type: String = row.try_get("bot_type")?;
        let bot_version: String = row.try_get("bot_version")?;
        let github: Option<String> = row.try_get("github")?;
        let owner: i64 = row.try_get("owner")?;

        Ok(Self {
            name,
            description,
            short_description,
            version: Version::parse(&version).unwrap(),
            author,
            owner,
            bot_type: BotType::try_from(bot_type).unwrap(),
            bot_version: VersionReq::parse(&bot_version).unwrap(),
            github,
        })
    }
}

pub fn load_package_manifest(input: &str) -> Result<PackageManifest, Error> {
    from_str(input)
}

#[test]
fn test_loads_valid() {
    use semver::AlphaNumeric;

    let input = r#"
    name = "EXPORT_MODULE"
    description = "Exports stuff"
    version = "1.0.0-pre"
    author = "Nadyita <nadyita@hodorraid.org>"
    bot_type = "Nadybot"
    bot_version = "^5.0.0"
    "#;
    let expected = PackageManifest {
        name: String::from("EXPORT_MODULE"),
        description: String::from("Exports stuff"),
        version: Version {
            major: 1,
            minor: 0,
            patch: 0,
            pre: vec![AlphaNumeric(String::from("pre"))],
            build: vec![],
        },
        author: String::from("Nadyita <nadyita@hodorraid.org>"),
        bot_type: BotType::Nadybot,
        bot_version: VersionReq::parse("^5.0.0").unwrap(), // Op is not exposed, cannot hardcode
        github: None,
    };
    assert_eq!(load_package_manifest(input).unwrap(), expected);
}
