// README.md to HTML converter
use pulldown_cmark::{html, Options, Parser};

pub fn to_html(markdown: &str) -> String {
    let options = Options::all();
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
