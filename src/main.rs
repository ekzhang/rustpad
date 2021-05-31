use warp::{filters::BoxedFilter, Filter, Reply};

fn server() -> BoxedFilter<(impl Reply,)> {
    (warp::path!("hello" / String).map(|name| format!("Hello, {}!", name)))
        .or(warp::path::end().map(|| "Home page"))
        .boxed()
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
