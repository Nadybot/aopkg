use crate::{
    manifest::{BotType, PackageDb, PackageVersion, PackageVersionDb, Requirement, RequirementDb},
    package::Package,
};

use actix_web::web::{Bytes, Data};
use s3::Bucket;
use semver::{Version, VersionReq};
use sqlx::MySqlPool;

use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
    S3(s3::error::S3Error),
    Unauthorized,
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<s3::error::S3Error> for Error {
    fn from(value: s3::error::S3Error) -> Self {
        Self::S3(value)
    }
}

pub fn validate_data(package: &Package) -> bool {
    let gh = package.manifest.github.clone();
    if let Some(g) = gh {
        if g.len() > 40 || g.split('/').count() != 2 {
            return false;
        }
    }

    package.manifest.author.len() <= 30
        && package.manifest.name.len() <= 30
        && package.manifest.description.len() <= 100
        && package.description.len() <= 8000
        && package.manifest.version.to_string().len() <= 12
        && package.manifest.bot_version.to_string().len() <= 50
        && package.manifest.bot_type.to_string().len() <= 15
        && package.manifest.requires.len() < 100
        && package
            .manifest
            .github
            .as_ref()
            .map_or(true, |github| github.len() <= 40)
        && package
            .manifest
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub async fn get_package_with_version(
    pool: Data<MySqlPool>,
    name: &str,
    version: &Version,
) -> Result<PackageVersion, Error> {
    let version_str = version.to_string();

    let data = sqlx::query_as!(
        PackageVersionDb,
        r#"
SELECT
    v.`id`,
    v.`package`,
    v.`description`,
    v.`short_description`,
    v.`author`,
    v.`version`,
    v.`bot_version`,
    p.`name`,
    v.`github`
FROM versions AS v
JOIN packages AS p
ON (v.`package`=p.`id`)
WHERE p.`name`=? AND v.`version`=?;"#,
        name,
        &version_str
    )
    .fetch_one(&**pool)
    .await?;

    let requirements = sqlx::query_as!(
        RequirementDb,
        r#"
SELECT
    `package_version`,
    `name`,
    `version`
FROM requirements
WHERE `package_version`=?;"#,
        data.id
    )
    .fetch_all(&**pool)
    .await?;

    Ok(PackageVersion {
        name: data.name,
        description: data.description,
        short_description: data.short_description,
        version: Version::parse(&data.version).unwrap(),
        author: data.author,
        bot_type: BotType::Nadybot,
        bot_version: VersionReq::parse(&data.bot_version).unwrap(),
        github: data.github,
        requires: requirements
            .into_iter()
            .map(|i| Requirement {
                name: i.name,
                version: VersionReq::parse(&i.version).unwrap(),
            })
            .collect(),
    })
}

pub async fn get_latest_package(
    pool: Data<MySqlPool>,
    name: &str,
) -> Result<PackageVersion, Error> {
    get_package_versions(pool, name)
        .await?
        .drain(0..1)
        .next()
        .ok_or(Error::Sqlx(sqlx::Error::RowNotFound))
}

pub async fn get_package_versions(
    pool: Data<MySqlPool>,
    name: &str,
) -> Result<Vec<PackageVersion>, Error> {
    let packages = sqlx::query_as!(
        PackageVersionDb,
        r#"
SELECT
    v.`id`,
    v.`package`,
    v.`description`,
    v.`short_description`,
    v.`author`,
    v.`version`,
    v.`bot_version`,
    p.`name`,
    v.`github`
FROM versions AS v
JOIN packages AS p
ON (v.`package`=p.`id`)
WHERE p.`name`=?
ORDER BY v.`id` DESC;"#,
        name
    )
    .fetch_all(&**pool)
    .await?;

    let requirements = sqlx::query_as!(
        RequirementDb,
        r#"
SELECT
    r.`package_version`,
    r.`name`,
    r.`version`
FROM requirements AS r
WHERE r.`package_version` IN (
    SELECT
        DISTINCT v.`id`
    FROM versions AS v
    JOIN packages AS p
    ON (v.`package`=p.`id`)
    WHERE p.`name`=?
)
ORDER BY r.`package_version` DESC;
        "#,
        name
    )
    .fetch_all(&**pool)
    .await?;

    let mut package_versions = Vec::with_capacity(packages.len());
    let mut requirements = requirements.into_iter();

    for package in packages {
        let requires = requirements
            .by_ref()
            .take_while(|i| i.package_version == package.id)
            .map(|i| Requirement {
                name: i.name,
                version: VersionReq::parse(&i.version).unwrap(),
            })
            .collect();

        package_versions.push(PackageVersion {
            name: package.name,
            description: package.description,
            short_description: package.short_description,
            version: Version::parse(&package.version).unwrap(),
            author: package.author,
            bot_type: BotType::Nadybot,
            bot_version: VersionReq::parse(&package.bot_version).unwrap(),
            github: package.github,
            requires,
        });
    }

    package_versions.sort_unstable_by(|a, b| b.version.cmp(&a.version));

    Ok(package_versions)
}

pub async fn get_all_packages(pool: Data<MySqlPool>) -> Result<Vec<PackageVersion>, Error> {
    let packages = sqlx::query_as!(
        PackageVersionDb,
        r#"
SELECT
    v.`id`,
    v.`package`,
    v.`description`,
    v.`short_description`,
    v.`author`,
    v.`version`,
    v.`bot_version`,
    p.`name`,
    v.`github`
FROM versions AS v
JOIN packages AS p
ON (v.`package`=p.`id`)
ORDER BY p.`id` DESC, v.`id` DESC;"#
    )
    .fetch_all(&**pool)
    .await?;

    let requirements = sqlx::query_as!(
        RequirementDb,
        r#"
SELECT
    `package_version`,
    `name`,
    `version`
FROM requirements;
        "#
    )
    .fetch_all(&**pool)
    .await?;
    let mut sorted_requirements: HashMap<i64, Vec<Requirement>> = HashMap::new();
    for requirement in requirements {
        sorted_requirements
            .entry(requirement.package_version)
            .or_default()
            .push(Requirement {
                name: requirement.name,
                version: VersionReq::parse(&requirement.version).unwrap(),
            });
    }

    Ok(packages
        .into_iter()
        .map(|package| PackageVersion {
            name: package.name,
            description: package.description,
            short_description: package.short_description,
            version: Version::parse(&package.version).unwrap(),
            author: package.author,
            bot_type: BotType::Nadybot,
            bot_version: VersionReq::parse(&package.bot_version).unwrap(),
            github: package.github,
            requires: sorted_requirements.remove(&package.id).unwrap_or_default(),
        })
        .collect())
}

pub async fn get_latest_packages(pool: Data<MySqlPool>) -> Result<Vec<PackageVersion>, Error> {
    let packages = sqlx::query_as!(
        PackageVersionDb,
        r#"
SELECT
    v.`id`,
    v.`package`,
    v.`description`,
    v.`short_description`,
    v.`author`,
    v.`version`,
    v.`bot_version`,
    p.`name`,
    v.`github`
FROM versions AS v
JOIN packages AS p
ON (v.`package`=p.`id`)
ORDER BY p.`id` DESC;"#
    )
    .fetch_all(&**pool)
    .await?;

    let mut latest_db_packages = Vec::new();
    let mut packages = packages.into_iter().peekable();

    while let Some(package_version) = packages.peek().map(|p| p.package) {
        let (mut same_package, other_packages): (Vec<_>, Vec<_>) =
            packages.partition(|i| i.package == package_version);

        same_package.sort_unstable_by(|a, b| a.version.cmp(&b.version));
        latest_db_packages.push(same_package.pop().unwrap());

        packages = other_packages.into_iter().peekable();
    }

    let requirements = sqlx::query_as!(
        RequirementDb,
        r#"
SELECT
    `package_version`,
    `name`,
    `version`
FROM requirements;
        "#
    )
    .fetch_all(&**pool)
    .await?;
    let mut sorted_requirements: HashMap<i64, Vec<Requirement>> = HashMap::new();
    for requirement in requirements {
        sorted_requirements
            .entry(requirement.package_version)
            .or_default()
            .push(Requirement {
                name: requirement.name,
                version: VersionReq::parse(&requirement.version).unwrap(),
            });
    }

    Ok(latest_db_packages
        .into_iter()
        .map(|package| PackageVersion {
            name: package.name,
            description: package.description,
            short_description: package.short_description,
            version: Version::parse(&package.version).unwrap(),
            author: package.author,
            bot_type: BotType::Nadybot,
            bot_version: VersionReq::parse(&package.bot_version).unwrap(),
            github: package.github,
            requires: sorted_requirements.remove(&package.id).unwrap_or_default(),
        })
        .collect())
}

pub async fn package_exists_for_repo(
    pool: Data<MySqlPool>,
    github: &str,
    owner: i64,
) -> Result<bool, Error> {
    let data = sqlx::query!(
        r#"
SELECT EXISTS(
    SELECT *
    FROM packages AS p
    JOIN versions AS v ON (p.`id`=v.`package`)
    WHERE v.`github`=? AND p.`owner`=?
) AS `package_exists`;"#,
        github,
        owner
    )
    .fetch_one(&**pool)
    .await?;

    Ok(data.package_exists != 0)
}

pub async fn create_package(
    pool: Data<MySqlPool>,
    bucket: Data<Bucket>,
    package: Package,
    owner_id: i64,
    file: Bytes,
) -> Result<(), Error> {
    let version = package.manifest.version.to_string();
    let bot_version = package.manifest.bot_version.to_string();

    bucket
        .put_object(
            format!(
                "{}-{}.zip",
                &package.manifest.name, &package.manifest.version
            ),
            &file,
        )
        .await?;

    let pkg = sqlx::query_as!(
        PackageDb,
        r#"SELECT `id`, `owner` FROM packages WHERE `name`=?;"#,
        &package.manifest.name
    )
    .fetch_optional(&**pool)
    .await?;

    let pkg_id = if let Some(p) = pkg {
        if p.owner != owner_id {
            return Err(Error::Unauthorized);
        }
        p.id
    } else {
        sqlx::query!(
            r#"INSERT INTO packages (`name`, `owner`) VALUES (?, ?);"#,
            &package.manifest.name,
            owner_id
        )
        .execute(&**pool)
        .await?
        .last_insert_id() as i64
    };

    sqlx::query!(
        r#"DELETE FROM versions WHERE `package`=? AND `version`=?;"#,
        &pkg_id,
        &version
    )
    .execute(&**pool)
    .await?;

    let package_version = sqlx::query!(
        r#"
INSERT INTO
    versions
(
    `package`,
    `description`,
    `short_description`,
    `version`,
    `author`,
    `bot_version`,
    `github`
)
VALUES (?, ?, ?, ?, ?, ?, ?);"#,
        pkg_id,
        package.description,
        package.manifest.description,
        version,
        package.manifest.author,
        bot_version,
        package.manifest.github
    )
    .execute(&**pool)
    .await?
    .last_insert_id();

    for (name, version) in package.manifest.requires {
        sqlx::query!(
            r#"
INSERT INTO
    requirements
(
    `package_version`,
    `name`,
    `version`
) VALUES (?, ?, ?);"#,
            package_version,
            name,
            version.to_string()
        )
        .execute(&**pool)
        .await?;
    }

    Ok(())
}
