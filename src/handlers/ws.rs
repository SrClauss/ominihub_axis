use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use tracing::{info, warn};

use crate::AppState;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut receiver = state.broadcaster.subscribe();
    let (mut sender, mut reader) = socket.split();

    info!("WebSocket client connected");

    let send_task = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let msg = match serde_json::to_string(&event) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("Failed to serialize event: {}", e);
                            continue;
                        }
                    };

                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("WebSocket client lagged {} messages", n);
                }
            }
        }
    });

    while let Some(msg) = reader.next().await {
        match msg {
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    info!("WebSocket client disconnected");
}
