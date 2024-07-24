use std::{collections::HashMap, net::SocketAddr, sync::Arc};

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
    let app_state = Arc::new(AppState { tx });
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
        .on_upgrade(move |socket| handle_socket(socket, String::from("userId123")))
}

// In this struct we store what streams each user connection should listen to.
// We listen to stream, and when we get new message, it has room id in it.
// We store all users/connections belonging to this room in this struct and
// route the message to them.
pub struct ChatRoomStreamsService {
    user_id_to_rooms_ids_map: HashMap<String, Vec<String>>,
    room_id_to_user_ids_map: HashMap<String, Vec<String>>,
}

pub struct AppState {
    tx: broadcast::Sender<String>,
}

async fn handle_socket(mut socket: WebSocket, user_id: String) {
    let (mut sender, mut receiver) = socket.split();
    // Spawn the first task that will receive broadcast messages and send text
    // messages over the websocket to our client.
    let sender = Arc::new(RwLock::new(sender));
    let sender_pong = Arc::clone(&sender);
    let mut send_task = tokio::spawn(async move {
        loop {
            // In any websocket error, break loop.
            if sender_pong
                .blocking_write()
                .send(Message::Text(String::from("Pong from server")))
                .await
                .is_err()
            {
                break;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(0.2)).await;
                println!("Pong sent succesfully");
            }
        }
    });

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(message) => {
                let new_msg = message
                    .to_text()
                    .expect("Cant parse text from message")
                    .to_string();
                sender
                    .blocking_write()
                    .send(Message::from(new_msg))
                    .await
                    .expect("Unable to send message back");
                println!("{:?}", message)
            }
            Err(e) => println!("Client disconnected with err {:?}", e),
        }
    }
}
