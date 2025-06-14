use std::convert::Infallible;

use async_stream::stream;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::Stream;
use tokio::{
    net::TcpListener,
    sync::{Mutex, OnceCell, broadcast},
};
use tower_http::cors::{Any, CorsLayer};

static COUNTER: OnceCell<Mutex<u32>> = OnceCell::const_new();

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(1);
    let state = AppState { tx };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET]);

    let app = Router::new()
        .route("/", get(home_handler))
        .route("/sse", get(sse_handler))
        .with_state(state)
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    println!("Listening on port 3000");

    axum::serve(listener, app).await.unwrap();
}

async fn home_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut counter = COUNTER
        .get_or_init(async || Mutex::new(0))
        .await
        .lock()
        .await;

    *counter += 1;

    let _ = state.tx.send("".to_string());

    (StatusCode::OK, (*counter).to_string())
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.tx.subscribe();

    Sse::new(stream! {
        {
            let counter = *COUNTER.get().unwrap().lock().await;

            yield Ok(Event::default().data(counter.to_string()))
        }

        while let Ok(_) = rx.recv().await {
            let counter = *COUNTER.get().unwrap().lock().await;

            yield Ok(Event::default().data(counter.to_string()))
        }
    })
    .keep_alive(KeepAlive::default())
}
