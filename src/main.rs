use warp::{filters::BoxedFilter, Filter, Reply};

fn frontend() -> BoxedFilter<(impl Reply,)> {
    warp::fs::dir("dist")
        .or(warp::get().and(warp::fs::file("dist/index.html")))
        .boxed()
}

fn backend() -> BoxedFilter<(impl Reply,)> {
    warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name))
        .boxed()
}

fn server() -> BoxedFilter<(impl Reply,)> {
    warp::path("api").and(backend()).or(frontend()).boxed()
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let port = std::env::var("PORT")
        .unwrap_or("3030".to_string())
        .parse()
        .expect("Unable to parse PORT");

    println!("Server listening on http://localhost:{}", port);
    warp::serve(server()).run(([0, 0, 0, 0], port)).await;
}
