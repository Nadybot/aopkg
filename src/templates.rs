use crate::manifest::PackageManifestDb;

use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub logged_in: bool,
    pub packages: Vec<PackageManifestDb>,
}

#[derive(Template)]
#[template(path = "packages.html")]
pub struct PackagesTemplate<'a> {
    pub logged_in: bool,
    pub name: &'a str,
    pub packages: Vec<PackageManifestDb>,
}

#[derive(Template)]
#[template(path = "package.html")]
pub struct PackageTemplate {
    pub logged_in: bool,
    pub package: PackageManifestDb,
}

#[derive(Template)]
#[template(path = "faq.html")]
pub struct Faq {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "api.html")]
pub struct Api {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "upload.html")]
pub struct Upload {
    pub logged_in: bool,
}
