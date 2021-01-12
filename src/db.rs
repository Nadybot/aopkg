use crate::{
    manifest::{PackageDb, PackageManifestDb},
    package::Package,
};

use actix_web::web::{Bytes, Data};
use semver::Version;
use sqlx::{sqlite::SqliteDone, Error, SqlitePool};
use tokio::fs::write;

use std::path::Path;

#[inline(always)]
pub fn validate_data(package: &Package) -> bool {
    package.manifest.author.len() <= 30
        && package.manifest.name.len() <= 30
        && package.manifest.description.len() <= 100
        && package.description.len() <= 8000
        && package.manifest.version.to_string().len() <= 12
        && package.manifest.bot_version.to_string().len() <= 24
        && package.manifest.bot_type.to_string().len() <= 15
        && package
            .manifest
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub async fn get_package_with_version(
    pool: Data<SqlitePool>,
    name: String,
    version: Version,
) -> Result<PackageManifestDb, Error> {
    let version_str = version.to_string();

    let data: PackageManifestDb = sqlx::query_as(
        r#"SELECT v."description", v."short_description", v."author", v."version", v."bot_version", v."bot_type", p."name" FROM versions v JOIN packages p ON (v."package"=p."id") WHERE p."name"=? AND v."version"=?;"#,
    ).bind(&name).bind(&version_str).fetch_one(&**pool).await?;

    Ok(data)
}

pub async fn get_latest_package(
    pool: Data<SqlitePool>,
    name: String,
) -> Result<PackageManifestDb, Error> {
    let data: PackageManifestDb = sqlx::query_as(
        r#"SELECT v."description", v."short_description", v."author", v."version", v."bot_version", v."bot_type", p."name" FROM versions v JOIN packages p ON (v."package"=p."id") WHERE p."name"=? ORDER BY v."id" DESC LIMIT 1;"#,
    ).bind(&name).fetch_one(&**pool).await?;

    Ok(data)
}

pub async fn get_package_versions(
    pool: Data<SqlitePool>,
    name: &str,
) -> Result<Vec<PackageManifestDb>, Error> {
    let data: Vec<PackageManifestDb> = sqlx::query_as(
        r#"SELECT v."description", v."short_description", v."author", v."version", v."bot_version", v."bot_type", p."name" FROM versions v JOIN packages p ON (v."package"=p."id") WHERE p."name"=? ORDER BY v."id" DESC;"#,
    ).bind(&name).fetch_all(&**pool).await?;

    Ok(data)
}

pub async fn get_all_packages(pool: Data<SqlitePool>) -> Result<Vec<PackageManifestDb>, Error> {
    let data: Vec<PackageManifestDb> = sqlx::query_as(
        r#"SELECT v."description", v."short_description", v."author", v."version", v."bot_version", v."bot_type", p."name" FROM versions v JOIN packages p ON (v."package"=p."id") ORDER BY v."package", v."id" DESC;"#,
    ).fetch_all(&**pool).await?;

    Ok(data)
}

pub async fn get_latest_packages(pool: Data<SqlitePool>) -> Result<Vec<PackageManifestDb>, Error> {
    let data: Vec<PackageManifestDb> = sqlx::query_as(
        r#"SELECT v."description", v."short_description", v."author", v."version", v."bot_version", v."bot_type", p."name" FROM versions v JOIN packages p ON (v."package"=p."id") GROUP BY v."package" HAVING MAX(v."id");"#,
    ).fetch_all(&**pool).await?;

    Ok(data)
}

pub async fn create_package(
    pool: Data<SqlitePool>,
    package: Package,
    owner_id: i64,
    file: Bytes,
) -> Result<SqliteDone, Error> {
    let version = package.manifest.version.to_string();
    let bot_version = package.manifest.bot_version.to_string();
    let bot_type = package.manifest.bot_type.to_string();
    let path = Path::new("data").join(&format!(
        "{}-{}.zip",
        &package.manifest.name, &package.manifest.version
    ));
    write(path, file).await?;

    let pkg: Option<PackageDb> =
        sqlx::query_as(r#"SELECT "id", "owner" FROM packages WHERE "name"=?;"#)
            .bind(&package.manifest.name)
            .fetch_optional(&**pool)
            .await?;

    let pkg_id = {
        if let Some(p) = pkg {
            if p.owner != owner_id {
                return Err(Error::RowNotFound); // anything really
            }
            p.id
        } else {
            sqlx::query(r#"INSERT INTO packages ("name", "owner") VALUES (?, ?);"#)
                .bind(package.manifest.name)
                .bind(owner_id)
                .execute(&**pool)
                .await?
                .last_insert_rowid()
        }
    };

    sqlx::query(
        r#"INSERT INTO versions ("package", "description", "short_description", "version", "author", "bot_type", "bot_version") VALUES (?, ?, ?, ?, ?, ?, ?);"#,
    )
        .bind(pkg_id)
        .bind(package.description)
        .bind(package.manifest.description)
        .bind(version)
        .bind(package.manifest.author)
        .bind(bot_type)
        .bind(bot_version)
        .execute(&**pool)
        .await
}
