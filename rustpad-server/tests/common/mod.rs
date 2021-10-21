use anyhow::{anyhow, Result};
use serde_json::Value;
use warp::{filters::BoxedFilter, test::WsClient, Reply};

/// A test WebSocket client that sends and receives JSON messages.
pub struct JsonSocket(WsClient);

impl JsonSocket {
    pub async fn send(&mut self, msg: &Value) {
        self.0.send_text(msg.to_string()).await
    }

    pub async fn recv(&mut self) -> Result<Value> {
        let msg = self.0.recv().await?;
        let msg = msg.to_str().map_err(|_| anyhow!("non-string message"))?;
        Ok(serde_json::from_str(msg)?)
    }

    pub async fn recv_closed(&mut self) -> Result<()> {
        self.0.recv_closed().await.map_err(|e| e.into())
    }
}

/// Connect a new test client WebSocket.
pub async fn connect(
    filter: &BoxedFilter<(impl Reply + 'static,)>,
    id: &str,
) -> Result<JsonSocket> {
    let client = warp::test::ws()
        .path(&format!("/api/socket/{}", id))
        .handshake(filter.clone())
        .await?;
    Ok(JsonSocket(client))
}

/// Check the text route.
pub async fn expect_text(filter: &BoxedFilter<(impl Reply + 'static,)>, id: &str, text: &str) {
    let resp = warp::test::request()
        .path(&format!("/api/text/{}", id))
        .reply(filter)
        .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.body(), text);
}
