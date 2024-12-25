// Parser for the nadypkg.toml files
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sqlx::{Decode, FromRow};
use toml::{de::Error, from_str};

use std::{
    collections::HashMap,
    convert::TryFrom,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub enum BotType {
    Nadybot,
}

impl Display for BotType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Nadybot => write!(f, "Nadybot"),
        }
    }
}

impl TryFrom<String> for BotType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Nadybot" => Ok(Self::Nadybot),
            _ => Err("Unknown or unsupported bot type"),
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
    #[serde(default)]
    pub requires: HashMap<String, VersionReq>,
}

pub struct RequirementDb {
    pub package_version: i64,
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct Requirement {
    pub name: String,
    pub version: VersionReq,
}

#[derive(Serialize)]
pub struct PackageVersion {
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub version: Version,
    pub author: String,
    pub bot_type: BotType,
    pub bot_version: VersionReq,
    pub github: Option<String>,
    pub requires: Vec<Requirement>,
}

pub struct PackageVersionDb {
    pub id: i64,
    pub package: i64,
    pub name: String,
    pub description: String,
    pub short_description: String,
    pub version: String,
    pub author: String,
    pub bot_version: String,
    pub github: Option<String>,
}

#[derive(FromRow, Decode)]
pub struct PackageDb {
    pub id: i64,
    pub owner: i64,
}

pub fn load_package_manifest(input: &str) -> Result<PackageManifest, Error> {
    from_str(input)
}

#[test]
fn test_loads_valid() {
    use semver::{BuildMetadata, Prerelease};

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
            pre: Prerelease::new("pre").unwrap(),
            build: BuildMetadata::EMPTY,
        },
        author: String::from("Nadyita <nadyita@hodorraid.org>"),
        bot_type: BotType::Nadybot,
        bot_version: VersionReq::parse("^5.0.0").unwrap(), // Op is not exposed, cannot hardcode
        github: None,
        requires: HashMap::new(),
    };
    assert_eq!(load_package_manifest(input).unwrap(), expected);
}
