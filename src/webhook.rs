use actix_web::{
    client::Client,
    web::{Bytes, Data},
    Error,
};
use log::debug;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub browser_download_url: String,
    pub content_type: String,
}

#[derive(Deserialize, Debug)]
pub struct Release {
    pub zipball_url: String,
    pub assets: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
pub struct Sender {
    pub id: i64,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Deserialize, Debug)]
pub struct GithubReleaseWebhook {
    pub action: String,
    pub release: Release,
    pub repository: Repository,
    pub sender: Sender,
}

pub async fn get_latest_release(repo: &str, client: Data<Client>) -> Result<Bytes, Error> {
    let data: Vec<Release> = client
        .get(&format!("https://api.github.com/repos/{}/releases", repo))
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "aopkg")
        .send()
        .await?
        .json()
        .await?;

    debug!("Found releases for  {}: {:?}", repo, data);

    if let Some(release) = data.get(0) {
        let asset = release
            .assets
            .iter()
            .find(|a| a.content_type == "application/zip" || a.name.ends_with(".zip"));
        let url = if let Some(asset) = asset {
            &asset.browser_download_url
        } else {
            &release.zipball_url
        };
        debug!("Getting webhook zip from {}", url);

        let resp = client.get(url).header("User-Agent", "aopkg").send().await?;
        let location = resp.headers().get("Location");

        debug!(
            "Got response: {:?} ({}) with redirect to {:?}",
            resp,
            resp.status(),
            location
        );

        if let Some(url) = location {
            let bytes = client
                .get(url.to_str().unwrap())
                .header("User-Agent", "aopkg")
                .send()
                .await?
                .body()
                .await?;
            debug!("Received zip: {:?}", bytes);
            Ok(bytes)
        } else {
            Err(Error::from(()))
        }
    } else {
        Err(Error::from(()))
    }
}
