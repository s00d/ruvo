//! Simple chat over WebSocket at `/ws`.

use sova::{App, Html, Result, Ws, WsRouteExt};
use sova::Message;

const CHAT_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Sova WS Chat</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 40rem; margin: 2rem auto; }
    #log { border: 1px solid #ccc; min-height: 12rem; padding: .5rem; overflow-y: auto; }
    form { display: flex; gap: .5rem; margin-top: .5rem; }
    input { flex: 1; }
  </style>
</head>
<body>
  <h1>WebSocket chat</h1>
  <div id="log"></div>
  <form id="form">
    <input id="msg" autocomplete="off" placeholder="Message…" />
    <button type="submit">Send</button>
  </form>
  <script>
    const log = document.getElementById('log');
    const form = document.getElementById('form');
    const input = document.getElementById('msg');
    const ws = new WebSocket(`ws://${location.host}/ws`);
    ws.onmessage = (e) => {
      const p = document.createElement('p');
      p.textContent = e.data;
      log.append(p);
      log.scrollTop = log.scrollHeight;
    };
    form.onsubmit = (e) => {
      e.preventDefault();
      if (input.value) ws.send(input.value);
      input.value = '';
    };
  </script>
</body>
</html>"#;

#[tokio::main]
async fn main() -> Result<()> {

    let mut app = App::new();
    app.install(Ws::new());

    app.get("/", |_| async { Html(CHAT_HTML) });

    app.ws("/ws", |mut session| async move {
        let _room = session.join("chat");
        while let Some(Ok(msg)) = session.recv().await {
            if let Message::Text(text) = msg {
                session
                    .hub()
                    .broadcast("chat", Message::Text(text))
                    .await;
            }
        }
    });

    tracing::info!("chat on http://127.0.0.1:3000/ (ws /ws)");
    app.listen(3000).await
}
