//! Server backend for the Rustpad collaborative text editor

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use warp::{filters::BoxedFilter, Filter, Reply};

mod server;

/// Construct routes for static files from React
fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("build")
        .or(warp::get().and(warp::fs::file("build/index.html")))
        .boxed()
}

/// Construct backend routes, including WebSocket handlers
fn backend() -> BoxedFilter<(impl Reply,)> {
    server::routes()
}

/// A combined filter handling all server routes
fn server() -> BoxedFilter<(impl Reply,)> {
    warp::path("api").and(backend()).or(frontend()).boxed()
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| String::from("3030"))
        .parse()
        .expect("Unable to parse PORT");

    warp::serve(server()).run(([0, 0, 0, 0], port)).await;
}
