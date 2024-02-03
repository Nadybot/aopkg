use actix_files::{Files, NamedFile};
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::{
    cookie::Key,
    get, middleware, post,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use askama::Template;
use awc::Client;
use log::debug;
use semver::Version;
use serde_json::to_string_pretty;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use std::{
    env::{set_var, var},
    io::Cursor,
    path::Path,
    str::FromStr,
};

mod db;
mod description;
mod manifest;
mod oauth;
mod package;
mod templates;
mod webhook;

#[post("/upload")]
async fn upload_package(
    payload: web::Bytes,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    if let Ok(Some(id)) = session.get::<i64>("id") {
        let cur = Cursor::new(payload.clone());

        match package::try_parse(cur).await {
            Ok(pkg) => {
                if !db::validate_data(&pkg) {
                    return HttpResponse::BadRequest()
                        .body("Package format OK, but parts too long");
                }

                return match db::create_package(pool, pkg, id, payload).await {
                    Ok(_) => HttpResponse::Created().finish(),
                    Err(_) => HttpResponse::Forbidden().finish(),
                };
            }
            Err(e) => return HttpResponse::BadRequest().body(format!("{:?}", e)),
        };
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

#[get("/api/packages/{name}/{version}")]
async fn get_package_data(
    path: web::Path<(String, Version)>,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    let package = db::get_package_with_version(pool, &path.0, &path.1).await;

    match package {
        Ok(pkg) => HttpResponse::Ok()
            .content_type("application/json")
            .body(to_string_pretty(&pkg).unwrap()),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/api/packages/{name}")]
async fn get_package_versions(
    name: web::Path<String>,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    let packages = db::get_package_versions(pool, &name)
        .await
        .expect("DB error");

    if !packages.is_empty() {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(to_string_pretty(&packages).unwrap())
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[get("/api/packages")]
async fn get_all_package_data(pool: web::Data<SqlitePool>) -> impl Responder {
    let packages = db::get_all_packages(pool).await.expect("DB error");
    HttpResponse::Ok()
        .content_type("application/json")
        .body(to_string_pretty(&packages).unwrap())
}

#[get("/api/packages/{name}/{version}/download")]
async fn download_package(req: HttpRequest, path: web::Path<(String, Version)>) -> impl Responder {
    if path
        .0
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        let path = Path::new("data").join(format!("{}-{}.zip", path.0, path.1));

        match NamedFile::open(path) {
            Ok(f) => Ok(f.into_response(&req)),
            Err(_) => HttpResponse::NotFound().await,
        }
    } else {
        HttpResponse::NotFound().await
    }
}

#[get("/")]
async fn package_list(pool: web::Data<SqlitePool>, session: Session) -> impl Responder {
    let packages = db::get_latest_packages(pool).await.expect("DB error");
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));
    HttpResponse::Ok().content_type("text/html").body(
        templates::Index {
            packages,
            logged_in,
        }
        .render()
        .unwrap(),
    )
}

#[get("/faq")]
async fn faq(session: Session) -> impl Responder {
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Faq { logged_in }.render().unwrap())
}

#[get("/api")]
async fn api(session: Session) -> impl Responder {
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Api { logged_in }.render().unwrap())
}

#[get("/upload")]
async fn upload_view(session: Session) -> impl Responder {
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Upload { logged_in }.render().unwrap())
}

#[get("/packages/{name}/{version}")]
async fn show_package_data(
    path: web::Path<(String, Version)>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let package = db::get_package_with_version(pool, &path.0, &path.1).await;
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    match package {
        Ok(pkg) => HttpResponse::Ok().content_type("text/html").body(
            templates::PackageTemplate {
                package: pkg,
                logged_in,
            }
            .render()
            .unwrap(),
        ),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/packages/{name}/latest")]
async fn show_latest_package_data(
    name: web::Path<String>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let package = db::get_latest_package(pool, &name).await;
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    match package {
        Ok(pkg) => HttpResponse::Ok().content_type("text/html").body(
            templates::PackageTemplate {
                package: pkg,
                logged_in,
            }
            .render()
            .unwrap(),
        ),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/packages/{name}")]
async fn show_package_version_data(
    name: web::Path<String>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let packages = db::get_package_versions(pool, &name).await;
    let logged_in = matches!(session.get::<i64>("id"), Ok(Some(_)));

    match packages {
        Ok(pkgs) => HttpResponse::Ok().content_type("text/html").body(
            templates::PackagesTemplate {
                packages: pkgs,
                name: &name,
                logged_in,
            }
            .render()
            .unwrap(),
        ),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/login")]
async fn login() -> impl Responder {
    HttpResponse::Found()
        .append_header(("Location", oauth::OAUTH_URL.clone()))
        .finish()
}

#[get("/github")]
async fn redirected_back(
    code: web::Query<oauth::QueryGithub>,
    client: web::Data<Client>,
    session: Session,
) -> Result<HttpResponse, actix_web::Error> {
    let access_token = oauth::get_access_token(&code.code, client.clone()).await?;
    let user_id = oauth::get_user(&access_token, client).await?;
    session.insert("id", user_id)?;

    Ok(HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish())
}

#[post("/webhook")]
async fn github_webhook(
    web::Json(data): web::Json<webhook::GithubReleaseWebhook>,
    client: web::Data<Client>,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    if data.action != "published" {
        debug!("{:?}: Not a publish release event, ignoring.", data);
        return HttpResponse::NoContent().finish();
    }

    let db_pkg = db::get_package_by_repo(pool.clone(), data.repository.full_name, data.sender.id)
        .await
        .expect("DB error");

    if let Some(p) = db_pkg {
        if let Ok(payload) = webhook::get_latest_release(&p.github.unwrap(), client).await {
            let cur = Cursor::new(payload.clone());

            match package::try_parse(cur).await {
                Ok(pkg) => {
                    if !db::validate_data(&pkg) {
                        return HttpResponse::BadRequest()
                            .body("Package format OK, but parts too long");
                    }

                    return match db::create_package(pool, pkg, p.owner, payload).await {
                        Ok(_) => HttpResponse::Created().finish(),
                        Err(_) => HttpResponse::Forbidden().finish(),
                    };
                }
                Err(e) => {
                    debug!("Error parsing package: {:?}", e);
                    return HttpResponse::BadRequest().body(format!("{:?}", e));
                }
            };
        } else {
            return HttpResponse::NotFound().body("no release found");
        }
    } else {
        return HttpResponse::NotFound().body("no package found");
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    if var("RUST_LOG").is_err() {
        set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let conn_options = SqliteConnectOptions::from_str(&var("DATABASE_URL").unwrap())
        .unwrap()
        .collation("semver_collation", |a, b| {
            Version::parse(a).unwrap().cmp(&Version::parse(b).unwrap())
        });

    let pool = SqlitePoolOptions::new()
        .connect_with(conn_options)
        .await
        .expect("Could not connect to sqlite db");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migration failed");

    let key = Key::derive_from(
        var("COOKIE_SECRET")
            .expect("COOKIE_SECRET is not set")
            .as_bytes(),
    );

    HttpServer::new(move || {
        let client = Client::builder()
            .wrap(awc::middleware::Redirect::new())
            .finish();

        App::new()
            .app_data(Data::new(pool.clone()))
            .app_data(Data::new(client))
            .app_data(web::PayloadConfig::new(15728640))
            .wrap(middleware::Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                key.clone(),
            ))
            .service(Files::new("/assets", "./static"))
            .service(upload_package)
            .service(download_package)
            .service(get_package_data)
            .service(get_package_versions)
            .service(get_all_package_data)
            .service(package_list)
            .service(faq)
            .service(api)
            .service(upload_view)
            .service(show_latest_package_data)
            .service(show_package_data)
            .service(show_package_version_data)
            .service(login)
            .service(redirected_back)
            .service(github_webhook)
    })
    .bind("0.0.0.0:7575")?
    .run()
    .await
}
