use rustpad_server::{database::Database, runner::GoJudgeClient, server, ServerConfig};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| String::from("3030"))
        .parse()
        .expect("Unable to parse PORT");

    let config = ServerConfig {
        expiry_days: std::env::var("EXPIRY_DAYS")
            .unwrap_or_else(|_| String::from("1"))
            .parse()
            .expect("Unable to parse EXPIRY_DAYS"),
        database: match std::env::var("SQLITE_URI") {
            Ok(uri) => Some(
                Database::new(&uri)
                    .await
                    .expect("Unable to connect to SQLITE_URI"),
            ),
            Err(_) => None,
        },
        runner: non_empty_var("GO_JUDGE_URL").map(|url| {
            GoJudgeClient::new(&url, non_empty_var("GO_JUDGE_TOKEN"))
                .expect("Unable to parse GO_JUDGE_URL")
        }),
    };

    warp::serve(server(config)).run(([0, 0, 0, 0], port)).await;
}

/// Reads an environment variable; empty counts as unset.
fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}
