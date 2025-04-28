//! Entrypoint.

use inserter::ClickhouseClient;

fn main() {
    extractor::extractor();
    ClickhouseClient::new("http://localhost:8123");
}
