#[macro_use]
extern crate finchers;
extern crate finchers_urlencoded;
extern crate http;
#[macro_use]
extern crate serde_derive;

use finchers::Application;
use finchers::endpoint::prelude::*;
use finchers_urlencoded::{form_body, from_csv, queries};
use std::fmt;

#[derive(Debug, Deserialize, HttpResponse)]
pub struct FormParam {
    query: String,
    count: Option<usize>,
    #[serde(deserialize_with = "from_csv")]
    tags: Option<Vec<String>>,
}

impl fmt::Display for FormParam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#}", self)
    }
}

fn main() {
    let endpoint = endpoint("search")
        .with(choice![get(queries()), post(form_body()),])
        .map(|param: FormParam| {
            println!("Received: {:#}", param);
            param
        });

    Application::from_endpoint(endpoint).run();
}
