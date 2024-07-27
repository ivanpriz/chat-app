use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{
    headers::{self, SecWebsocketKey},
    TypedHeader,
};
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::sync::{broadcast, RwLock};

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);
    let app_state = Arc::new(AppState {
        user_set: Mutex::new(HashSet::new()),
        tx,
    });
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn auth_user(auth_header: Option<&HeaderValue>) -> String {
    return "user_id".into();
}

pub struct AppState {
    tx: broadcast::Sender<String>,
    user_set: Mutex<HashSet<String>>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    println!("Got request to connect to ws");
    let sec_websocket_protocol = headers
        .get("Sec-Websocket-Protocol")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    println!(
        "Websocket connected, auth header: {:?}",
        sec_websocket_protocol
    );
    // Here some authentication will happen.
    // After successful auth we get user id,
    // By this user id we can get all the rooms the user is at.
    // let user_id = auth_user(sec_websocket_protocol);
    ws.protocols([sec_websocket_protocol])
        .on_upgrade(move |socket| handle_socket(socket, String::from("userId123"), state))
}

fn check_username(state: &AppState, string: &mut String, name: &str) {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        string.push_str(name);
    }
}

async fn handle_socket(mut socket: WebSocket, user_id: String, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.tx.subscribe();

    let msg = format!("{user_id} joined.");
    println!("{}", msg);
    let _ = state.tx.send(msg);

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let mut tx = state.tx.clone();
    let id_clone = user_id.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            println!("Received {} from user {}.", text, user_id);
            let _ = tx.send(format!("{user_id}: {text}"));
        }
    });

    // If any of the task completes, we abort the other (why?)
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    // Send user left
    let msg = format!("{id_clone} left");
    println!("{}", msg);
    let _ = state.tx.send(msg);

    state.user_set.lock().unwrap().remove(&id_clone);
}
