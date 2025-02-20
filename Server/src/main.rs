use crate::config::settings::Config;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use scylla::{CachingSession, Session, SessionBuilder};
use std::env;
use std::sync::Arc;
use actix_files::Files;
use tera::Tera;
use actix_web::middleware::Logger;
use actix_session::{SessionMiddleware, storage::RedisSessionStore};
use actix_web::cookie::Key;

mod config;
mod db;
mod handlers;
mod models;
mod routes;
mod utils;
use env_logger;

use crate::db::init::initialize_schema;
use crate::db::scylla_db_operations::ScyllaDbOperations;
use crate::routes::api;

struct RedisAppState {
    redis_client: Arc<redis::Client>,
}

fn create_templates() -> Tera {
    let mut tera = Tera::new("templates/**/*.html").expect("failed to parse template");
    tera.autoescape_on(vec![".html"]);
    tera
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ENV Config
    let config = match Config::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Scylla Connection

    let scylla_host: String = env::var("SCYLLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let session: Session = SessionBuilder::new()
        .known_node(scylla_host)
        .build()
        .await
        .expect("Failed to connect to ScyllaDB");
    // Init Scylla DB
    if let Err(err) = initialize_schema(&session, "resources/init.sql").await {
        eprintln!("Failed to initialize schema: {:?}", err);
    }
    // Scylla db Operations Handler
    let db_ops = web::Data::new(ScyllaDbOperations::new(
        Arc::new(CachingSession::from(session, 32)).clone(),
    ));

    // Redis Connection
    let redis_host: String =
        env::var("REDIS_HOST").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let redis_client =
        Arc::new(redis::Client::open(redis_host.clone()).expect("Failed to open Redis client"));
    let redis_app_state = web::Data::new(RedisAppState {
        redis_client: redis_client.clone(),
    });

    // std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    // actix session
    let secret_key = Key::generate();
    let redis_store = RedisSessionStore::new(redis_host)
        .await
        .unwrap();

    println!("Starting Now");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::permissive(), // This allows all origins, methods, and headers
            )
            .wrap(
                SessionMiddleware::new(
                    redis_store.clone(),
                    secret_key.clone(),
                )
            )
            .service(Files::new("/static", "./templates/static").show_files_listing())
            .app_data(db_ops.clone())
            .app_data(web::Data::new(config.clone()).clone())
            .app_data(redis_app_state.clone())
            .app_data(web::Data::new(create_templates()))
            .configure(api::configure)
    })
    .bind(("0.0.0.0", 8181))?
    .run()
    .await
}
