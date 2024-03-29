use actix_web::{error::ErrorInternalServerError, web::Data, Error};
use awc::Client;
use lazy_static::lazy_static;
use serde::Deserialize;

use std::env::var;

lazy_static! {
    pub static ref CLIENT_ID: String = var("CLIENT_ID").unwrap();
    pub static ref CLIENT_SECRET: String = var("CLIENT_SECRET").unwrap();
    pub static ref OAUTH_URL: String = format!(
        "https://github.com/login/oauth/authorize?client_id={}",
        *CLIENT_ID
    );
}

#[inline(always)]
pub fn access_token_url(code: &str) -> String {
    format!(
        "https://github.com/login/oauth/access_token?client_id={}&client_secret={}&code={}",
        *CLIENT_ID, *CLIENT_SECRET, code
    )
}

#[derive(Deserialize)]
pub struct QueryGithub {
    pub code: String,
}

#[derive(Deserialize)]
struct User {
    id: i64,
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
}

pub async fn get_access_token(code: &str, client: Data<Client>) -> Result<String, Error> {
    let data: AccessToken = client
        .get(access_token_url(code))
        .insert_header(("Accept", "application/json"))
        .send()
        .await
        .map_err(ErrorInternalServerError)?
        .json()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(data.access_token)
}

pub async fn get_user(access_token: &str, client: Data<Client>) -> Result<i64, Error> {
    let data: User = client
        .get("https://api.github.com/user")
        .insert_header(("Authorization", format!("token {}", access_token)))
        .insert_header(("Accept", "application/vnd.github.v3+json"))
        .insert_header(("User-Agent", "aopkg"))
        .send()
        .await
        .map_err(ErrorInternalServerError)?
        .json()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(data.id)
}
