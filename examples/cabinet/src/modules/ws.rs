use sova::{Message, Router, WsRouteExt};

pub fn mount(r: &mut Router) {
    r.ws("/ws", |mut session| async move {
        let _ = session
            .send(Message::Text("connected — type a message to echo in your room".into()))
            .await;
        let _room = session.join("cabinet");
        while let Some(Ok(msg)) = session.recv().await {
            if let Message::Text(text) = msg {
                let reply = format!("echo: {text}");
                session
                    .hub()
                    .broadcast("cabinet", Message::Text(reply.into()))
                    .await;
            }
        }
    });
}
