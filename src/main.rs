use actix_files::{Files, NamedFile};
use actix_session::{CookieSession, Session};
use actix_web::{
    client::Client, get, middleware, post, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use askama::Template;
use semver::Version;
use serde_json::to_string_pretty;
use sqlx::{migrate::Migrator, SqlitePool};

use std::{
    env::{set_var, var},
    io::Cursor,
    path::Path,
};

mod db;
mod description;
mod manifest;
mod oauth;
mod package;
mod templates;

#[post("/upload")]
async fn upload_package(
    payload: web::Bytes,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    if let Some(id) = session.get::<i64>("id")? {
        let cur = Cursor::new(payload.clone());

        match package::try_parse(cur).await {
            Ok(pkg) => {
                if !db::validate_data(&pkg) {
                    return HttpResponse::BadRequest()
                        .body("Package format OK, but parts too long")
                        .await;
                }

                return match db::create_package(pool, pkg, id, payload).await {
                    Ok(_) => HttpResponse::Created().await,
                    Err(_) => HttpResponse::Forbidden().await,
                };
            }
            Err(e) => return HttpResponse::BadRequest().body(format!("{:?}", e)).await,
        };
    } else {
        HttpResponse::Unauthorized().await
    }
}

#[get("/api/packages/{name}/{version}")]
async fn get_package_data(
    web::Path((name, version)): web::Path<(String, Version)>,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    let package = db::get_package_with_version(pool, name, version).await;

    match package {
        Ok(pkg) => {
            HttpResponse::Ok()
                .content_type("application/json")
                .body(to_string_pretty(&pkg).unwrap())
                .await
        }
        Err(_) => HttpResponse::NotFound().await,
    }
}

#[get("/api/packages/{name}")]
async fn get_package_versions(
    web::Path(name): web::Path<String>,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    let packages = db::get_package_versions(pool, &name)
        .await
        .expect("DB error");

    if !packages.is_empty() {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(to_string_pretty(&packages).unwrap())
            .await
    } else {
        HttpResponse::NotFound().await
    }
}

#[get("/api/packages")]
async fn get_all_package_data(pool: web::Data<SqlitePool>) -> impl Responder {
    let packages = db::get_all_packages(pool).await.expect("DB error");
    HttpResponse::Ok()
        .content_type("application/json")
        .body(to_string_pretty(&packages).unwrap())
        .await
}

#[get("/api/packages/{name}/{version}/download")]
async fn download_package(
    req: HttpRequest,
    web::Path((name, version)): web::Path<(String, Version)>,
) -> impl Responder {
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        let path = Path::new("data").join(format!("{}-{}.zip", name, version));

        match NamedFile::open(path) {
            Ok(f) => f.into_response(&req),
            Err(_) => HttpResponse::NotFound().await,
        }
    } else {
        HttpResponse::NotFound().await
    }
}

#[get("/")]
async fn package_list(pool: web::Data<SqlitePool>, session: Session) -> impl Responder {
    let packages = db::get_latest_packages(pool).await.expect("DB error");
    let logged_in = session.get::<i64>("id")?.is_some();
    HttpResponse::Ok()
        .content_type("text/html")
        .body(
            templates::Index {
                packages,
                logged_in,
            }
            .render()
            .unwrap(),
        )
        .await
}

#[get("/faq")]
async fn faq(session: Session) -> impl Responder {
    let logged_in = session.get::<i64>("id")?.is_some();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Faq { logged_in }.render().unwrap())
        .await
}

#[get("/api")]
async fn api(session: Session) -> impl Responder {
    let logged_in = session.get::<i64>("id")?.is_some();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Api { logged_in }.render().unwrap())
        .await
}

#[get("/upload")]
async fn upload_view(session: Session) -> impl Responder {
    let logged_in = session.get::<i64>("id")?.is_some();

    HttpResponse::Ok()
        .content_type("text/html")
        .body(templates::Upload { logged_in }.render().unwrap())
        .await
}

#[get("/packages/{name}/{version}")]
async fn show_package_data(
    web::Path((name, version)): web::Path<(String, Version)>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let package = db::get_package_with_version(pool, name, version).await;
    let logged_in = session.get::<i64>("id")?.is_some();

    match package {
        Ok(pkg) => {
            HttpResponse::Ok()
                .content_type("text/html")
                .body(
                    templates::PackageTemplate {
                        package: pkg,
                        logged_in,
                    }
                    .render()
                    .unwrap(),
                )
                .await
        }
        Err(_) => HttpResponse::NotFound().await,
    }
}

#[get("/packages/{name}/latest")]
async fn show_latest_package_data(
    web::Path(name): web::Path<String>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let package = db::get_latest_package(pool, name).await;
    let logged_in = session.get::<i64>("id")?.is_some();

    match package {
        Ok(pkg) => {
            HttpResponse::Ok()
                .content_type("text/html")
                .body(
                    templates::PackageTemplate {
                        package: pkg,
                        logged_in,
                    }
                    .render()
                    .unwrap(),
                )
                .await
        }
        Err(_) => HttpResponse::NotFound().await,
    }
}

#[get("/packages/{name}")]
async fn show_package_version_data(
    web::Path(name): web::Path<String>,
    pool: web::Data<SqlitePool>,
    session: Session,
) -> impl Responder {
    let packages = db::get_package_versions(pool, &name).await;
    let logged_in = session.get::<i64>("id")?.is_some();

    match packages {
        Ok(pkgs) => {
            HttpResponse::Ok()
                .content_type("text/html")
                .body(
                    templates::PackagesTemplate {
                        packages: pkgs,
                        name: &name,
                        logged_in,
                    }
                    .render()
                    .unwrap(),
                )
                .await
        }
        Err(_) => HttpResponse::NotFound().await,
    }
}

#[get("/login")]
async fn login() -> impl Responder {
    HttpResponse::Found()
        .header("Location", oauth::OAUTH_URL.clone())
        .await
}

#[get("/github")]
async fn redirected_back(
    web::Query(code): web::Query<oauth::QueryGithub>,
    client: web::Data<Client>,
    session: Session,
) -> impl Responder {
    let access_token = oauth::get_access_token(&code.code, client.clone()).await?;
    let user_id = oauth::get_user(&access_token, client).await?;
    session.set("id", user_id)?;

    HttpResponse::Found().header("Location", "/").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    if var("RUST_LOG").is_err() {
        set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    }
    env_logger::init();

    let m = Migrator::new(Path::new("./migrations")).await.unwrap();

    let pool = SqlitePool::connect(&var("DATABASE_URL").unwrap())
        .await
        .expect("Could not connect to sqlite db");

    let mut conn = pool.acquire().await.unwrap();
    conn.create_collation("semver_collation", |a, b| {
        Version::parse(a).unwrap().cmp(&Version::parse(b).unwrap())
    })
    .unwrap();
    drop(conn);

    m.run(&pool).await.expect("Migration failed");

    HttpServer::new(move || {
        let client = Client::new();

        App::new()
            .data(pool.clone())
            .data(client)
            .wrap(middleware::Logger::default())
            .app_data(web::PayloadConfig::new(5242880))
            .wrap(CookieSession::signed(
                var("COOKIE_SECRET").unwrap().as_bytes(),
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
    })
    .bind("0.0.0.0:7575")?
    .run()
    .await
}
