use std::convert::Infallible;

use async_stream::stream;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{
        Html, IntoResponse, Sse,
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

static VISITAS: OnceCell<Mutex<u32>> = OnceCell::const_new();
static CLIQUES: OnceCell<Mutex<u32>> = OnceCell::const_new();

#[derive(Clone, PartialEq)]
enum ServerSideEvent {
    Visita(u32),
    Clique(u32),
}

#[derive(Clone)]
struct AppState {
    sse: broadcast::Sender<ServerSideEvent>,
}

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel(1);
    let state = AppState { sse: tx };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET]);

    let app = Router::new()
        .route("/", get(home_handler))
        .route("/clique", get(clique_handler))
        .route("/sse/visita", get(sse_visita_handler))
        .route("/sse/clique", get(sse_clique_handler))
        .with_state(state)
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    println!("Listening on port 3000");

    axum::serve(listener, app).await.unwrap();
}

async fn home_handler(State(state): State<AppState>) -> Html<String> {
    tokio::spawn(async move {
        let mut visitas = VISITAS
            .get_or_init(async || Mutex::new(0))
            .await
            .lock()
            .await;

        *visitas += 1;

        let _ = state.sse.send(ServerSideEvent::Visita(*visitas));
    });

    Html(include_str!("index.html").to_string())
}

async fn clique_handler(State(state): State<AppState>) -> impl IntoResponse {
    tokio::spawn(async move {
        let mut cliques = CLIQUES
            .get_or_init(async || Mutex::new(0))
            .await
            .lock()
            .await;

        *cliques += 1;

        let _ = state.sse.send(ServerSideEvent::Clique(*cliques));
    });

    (StatusCode::OK, ())
}

async fn sse_visita_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.sse.subscribe();

    Sse::new(stream! {
        {
            let mut visitas = VISITAS
                .get_or_init(async || Mutex::new(0))
                .await
                .lock()
                .await;

            *visitas += 1;

            yield Ok(Event::default().data(visitas.to_string()))
        }

        while let Ok(event) = rx.recv().await {
            if let ServerSideEvent::Visita(visitas) = event {
                yield Ok(Event::default().data(visitas.to_string()))
            }
        }
    })
    .keep_alive(KeepAlive::default())
}

async fn sse_clique_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.sse.subscribe();

    Sse::new(stream! {
        {
            let mut cliques = CLIQUES
                .get_or_init(async || Mutex::new(0))
                .await
                .lock()
                .await;

            *cliques += 1;

            yield Ok(Event::default().data(cliques.to_string()))
        }

        while let Ok(event) = rx.recv().await {
            if let ServerSideEvent::Clique(cliques) = event {
                yield Ok(Event::default().data(cliques.to_string()))
            }
        }
    })
    .keep_alive(KeepAlive::default())
}
