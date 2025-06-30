use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use base58::ToBase58;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{Keypair, Signer};
use std::env;

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Deserialize)]
struct KeypairRequest {}

#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

async fn generate_keypair(_: web::Json<KeypairRequest>) -> impl Responder {
    let keypair = Keypair::new();
    let secret_bytes = keypair.to_bytes();
    let response = ApiResponse {
        success: true,
        data: Some(KeypairResponse {
            pubkey: keypair.pubkey().to_string(),
            secret: secret_bytes.to_base58(),
        }),
        error: None,
    };
    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let bind_address = format!("{}:{}", host, port);
    
    println!("Starting server at http://{}", bind_address);
    HttpServer::new(|| {
        App::new()
            .route("/keypair", web::post().to(generate_keypair))
    })
    .bind(bind_address)?
    .run()
    .await
}