//! Generate `OpenAPI` specification for the Taikoscope API

use api::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI spec");
    println!("{json}");
}
