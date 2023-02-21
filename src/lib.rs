use axum::{
    extract::{
        Extension, 
    },
    routing::{get, post},
    Router,
};

use dotenv::dotenv;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::{env, ffi::OsStr, sync::Arc, time::Duration};
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

use crate::handlers::voice_generation_handler::{
    generate, 
};
use crate::handlers::voice_spreadsheet_handler::{
    generate_audio, process_audio, detect_pauses
};
use crate::handlers::voice_extracts_handler::{
    extract_audio_transcripts, extract_transcripts_csv
};
use crate::handlers::voice_train_handler::{
    prepare_voice_data
};
use crate::handlers::video_lipsync_handler::{
    generate_video_lipsync
};
use crate::handlers::voice_huggingface_handler::{
    generate_audio_huggingface//, transfer_prosody
};
use crate::handlers::voice_mturk_handler::{
    fetch_mturk_data, process_mturk_data, upload_mturk_s3, login_mturk, signup_mturk, get_mturk_user, set_paypal, set_payment, fetch_users_info
};
use crate::utils::download_audio::{
    download_s3_names, clean_audio_db, voice_code, generate_folder_code, remove_folder_code, get_removed_names, detect_special_audios, detect_one_audio
};
use crate::models::ws_types::ServerState;

use microservice_utils::{
    server::{
        producer::get_producer, 
        error_404::error_404, 
    },
};

#[macro_use]
extern crate lazy_static;

fn ensure_var<K: AsRef<OsStr>>(key: K) -> anyhow::Result<String> {
    env::var(&key).map_err(|e| anyhow::anyhow!("{}: {:?}", e, key.as_ref()))
}

lazy_static! {
    static ref DATABASE_URL: String = ensure_var("DATABASE_URL").unwrap();
    static ref KAFKA_URL: String = ensure_var("KAFKA_URL").unwrap();
}

#[tokio::main]
pub async fn main() {
    dotenv().expect("Failed to read .env file");
    lazy_static::initialize(&DATABASE_URL);
    lazy_static::initialize(&KAFKA_URL);    

    // let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
    //     .await
    //     .unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&DATABASE_URL)    
        .await
        .unwrap();

    let axum_make_service = create_app(&pool);

    // let pool_arc = Arc::new(pool.clone());
    // let _result = download_s3_names(pool_arc).await;

    // let pool_arc = Arc::new(pool.clone());
    // let _result = generate_folder_code(pool_arc).await;
    
    // let path = "test/768b35b7-93a1-4956-abb0-1ee9628c02cf.wav".to_string();
    // detect_one_audio(&path);

    // let _clients: HashMap<String, Client> = HashMap::new();    
    // let clients = Arc::new(Mutex::new(_clients));
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
    .serve(axum_make_service.into_make_service())
    .await
    .unwrap(); 

    // let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    // println!("Listening on http://{}", addr);

    // axum_server::bind(addr)
    //     .serve(axum_make_service.into_make_service())
    //     .await
    //     .unwrap();

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

    let producer = Arc::new(get_producer(&KAFKA_URL));

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
        .route(
            "/generate_audio_huggingface",
            post(generate_audio_huggingface),
        )
        .route(
            "/voice_code",
            post(voice_code),
        )
        .route(
            "/get_removed_names",
            get(get_removed_names),
        )
        .route(
            "/process_audio",
            post(process_audio),
        )
        .route(
            "/detect_pauses",
            post(detect_pauses),
        )
        .route(
            "/fetch_mturk_data",
            post(fetch_mturk_data),
        )
        .route(
            "/process_mturk_data",
            post(process_mturk_data),
        )
        .route(
            "/upload_mturk_s3",
            post(upload_mturk_s3),
        )
        .route(
            "/extract_audio_transcripts",
            post(extract_audio_transcripts),
        )
        .route(
            "/extract_transcripts_csv",
            post(extract_transcripts_csv),
        )
        .route(
            "/login_mturk",
            post(login_mturk),
        )
        .route(
            "/signup_mturk",
            post(signup_mturk),
        )
        .route(
            "/get_mturk_user",
            post(get_mturk_user),
        ).route(
            "/set_paypal",
            post(set_paypal),
        ).route(
            "/set_payment",
            post(set_payment),
        ).route(
            "/fetch_users_info",
            post(fetch_users_info),
        )
        // .route(
        //     "/transfer_prosody",
        //     post(transfer_prosody),
        // )
        .route(
            "/generate_video_lipsync",
            post(generate_video_lipsync),
        )
        .route(
            "/prepare_voice_data",
            post(prepare_voice_data),
        )
        .route("/detect_scan", get(detect_scan))
        .fallback(get(error_404))
        .layer(Extension(state))
        .layer(Extension(pool_arc))
        .layer(Extension(producer))
        .layer(middleware_stack);

    return app;
}

async fn detect_scan() {
    let _res = detect_special_audios().await;
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
        for key in keys {
            state.documents.remove(&key);
        }
    }
}
