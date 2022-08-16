use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use rdkafka::consumer::{CommitMode, Consumer};
use sqlx::PgPool;
use std::{env, ffi::OsStr, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::{self};
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};

pub mod error_404;
pub mod handlers;
pub mod models;
pub mod utils;

use crate::error_404::error_404::error_404;
use crate::handlers::voice_generation_handler::{
    generate, 
};
use crate::handlers::voice_spreadsheet_handler::{
    generate_audio, 
};
use crate::utils::download_audio::{
    download_s3_names, clean_audio_db
};
use crate::models::ws_types::ServerState;
use crate::utils::consumer::get_consumer;

#[macro_use]
extern crate lazy_static;

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}

lazy_static! {
    static ref DATABASE_URL: String = ensure_var("DATABASE_URL").unwrap();
}

#[tokio::main]
pub async fn main() {
    dotenv().expect("Failed to read .env file");
    lazy_static::initialize(&DATABASE_URL);

    let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let axum_make_service = create_app(&pool);

    // let pool_arc = Arc::new(pool.clone());
    // let _result = download_s3_names(pool_arc).await;

    let pool_arc = Arc::new(pool.clone());
    let _result = clean_audio_db(pool_arc).await;

    start_consumer();

    
    // axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
    // .serve(axum_make_service.into_make_service())
    // .await
    // .unwrap(); 

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);

    axum_server::bind(addr)
        .serve(axum_make_service.into_make_service())
        .await
        .unwrap();

}

fn create_app(pool: &PgPool) -> Router {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            "example_websockets=debug,tower_http=debug,librdkafka=trace,rdkafka::client=debug",
        )
    }

    let state = ServerState {
        documents: Default::default(),
    };
    tokio::spawn(cleaner(state.clone(), 1));

    let pool_arc = Arc::new(pool.clone());

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_credentials(false)
        .allow_headers(Any);

    // Limit concurrency for all routes ,Trace layer for all routes
    let middleware_stack = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(ConcurrencyLimitLayer::new(64))
        .layer(cors)
        .into_inner();

    let app = Router::new()
        .route(
            "/voice_generate",
            post(generate),
        )
        .route(
            "/generate_audio",
            post(generate_audio),
        )
        // .route("/", get(hello_world))
        .fallback(get(error_404))
        .layer(Extension(state))
        .layer(Extension(pool_arc))
        .layer(middleware_stack);

    return app;
}

// async fn hello_world() -> String {
//     "Hello World".to_owned()
// }

fn start_consumer() {
    tokio::spawn(async move {
        let consumer = Arc::new(get_consumer("127.0.0.1:9092", "1234", &["bhuman_channel"]));
        loop {
            match consumer.recv().await {
                Err(e) => println!("Kafka error: {}", e),
                Ok(m) => {
                    let payload_s = match rdkafka::Message::payload_view::<str>(&m) {
                        None => "".to_string(),
                        Some(Ok(s)) => s.to_string(),
                        Some(Err(e)) => {
                            println!("Error while deserializing message payload: {:?}", e);
                            "".to_string()
                        }
                    };

                    println!("Received Message: {}", payload_s);

                    consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
            }
        }
    });
}

const HOUR: Duration = Duration::from_secs(3600);

/// Reclaims memory for documents.
async fn cleaner(state: ServerState, expiry_days: u32) {
    loop {
        time::sleep(HOUR).await;
        let mut keys = Vec::new();
        for entry in &*state.documents {
            if entry.last_accessed.elapsed() > HOUR * 24 * expiry_days {
                keys.push(entry.key().clone());
            }
        }
        println!("cleaner removing keys: {:?}", keys);
        for key in keys {
            state.documents.remove(&key);
        }
    }
}
