use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use base64::{engine::general_purpose, Engine as _};
use bs58;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::instruction as token_instruction;
use std::env;
use std::str::FromStr;

// --- Generic API Response Structures ---

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// A generic error response function to reduce boilerplate
fn error_response(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(ApiResponse::<()> {
        success: false,
        data: None,
        error: Some(msg.to_string()),
    })
}

// --- 1. Generate Keypair Endpoint ---

#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

/// Handles POST /keypair
/// Generates a new Solana keypair.
async fn generate_keypair() -> impl Responder {
    let keypair = Keypair::new();
    // A standard secret key is 32 bytes, which is the first half of the 64-byte array from to_bytes().
    let secret_key_bytes = &keypair.to_bytes()[..32];

    let response = ApiResponse {
        success: true,
        data: Some(KeypairResponse {
            pubkey: keypair.pubkey().to_string(),
            secret: bs58::encode(secret_key_bytes).into_string(),
        }),
        error: None,
    };
    HttpResponse::Ok().json(response)
}

// --- 2. Create Token Endpoint ---

#[derive(Deserialize)]
struct CreateTokenRequest {
    #[serde(rename = "mintAuthority")]
    mint_authority: String,
    mint: String,
    decimals: u8,
}

#[derive(Serialize)]
struct AccountInfo {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

#[derive(Serialize)]
struct InstructionResponse {
    program_id: String,
    accounts: Vec<AccountInfo>,
    #[serde(rename = "instruction_data")]
    instruction_data: String,
}

/// Handles POST /token/create
/// Creates an SPL Token `InitializeMint` instruction.
async fn create_token(req: web::Json<CreateTokenRequest>) -> impl Responder {
    // Parse mint authority public key from request
    let mint_authority_pubkey = match Pubkey::from_str(&req.mint_authority) {
        Ok(pubkey) => pubkey,
        Err(_) => return error_response("Invalid base58 string for mintAuthority."),
    };

    // Parse mint account public key from request
    let mint_pubkey = match Pubkey::from_str(&req.mint) {
        Ok(pubkey) => pubkey,
        Err(_) => return error_response("Invalid base58 string for mint."),
    };

    // Create the `InitializeMint` instruction
    let instruction = match token_instruction::initialize_mint(
        &spl_token::id(),
        &mint_pubkey,
        &mint_authority_pubkey,
        None, // No freeze authority
        req.decimals,
    ) {
        Ok(inst) => inst,
        Err(e) => return error_response(&format!("Failed to create instruction: {}", e)),
    };

    // Format the accounts for the JSON response
    let accounts = instruction
        .accounts
        .iter()
        .map(|acc| AccountInfo {
            pubkey: acc.pubkey.to_string(),
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect();

    // Build the final response data
    let response_data = InstructionResponse {
        program_id: instruction.program_id.to_string(),
        accounts,
        instruction_data: general_purpose::STANDARD.encode(&instruction.data),
    };

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
    })
}

// --- 3. Mint Token Endpoint ---

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

/// Handles POST /token/mint
/// Creates an SPL Token `MintTo` instruction.
async fn mint_token(req: web::Json<MintTokenRequest>) -> impl Responder {
    // Parse mint public key
    let mint_pubkey = match Pubkey::from_str(&req.mint) {
        Ok(pubkey) => pubkey,
        Err(_) => return error_response("Invalid base58 string for mint."),
    };

    // Parse destination public key
    let destination_pubkey = match Pubkey::from_str(&req.destination) {
        Ok(pubkey) => pubkey,
        Err(_) => return error_response("Invalid base58 string for destination."),
    };

    // Parse authority public key
    let authority_pubkey = match Pubkey::from_str(&req.authority) {
        Ok(pubkey) => pubkey,
        Err(_) => return error_response("Invalid base58 string for authority."),
    };

    // Create the `MintTo` instruction
    let instruction = match token_instruction::mint_to(
        &spl_token::id(),
        &mint_pubkey,
        &destination_pubkey,
        &authority_pubkey,
        &[], // No multisig signers
        req.amount,
    ) {
        Ok(inst) => inst,
        Err(e) => return error_response(&format!("Failed to create mint-to instruction: {}", e)),
    };

    // Format the accounts for the JSON response
    let accounts = instruction
        .accounts
        .iter()
        .map(|acc| AccountInfo {
            pubkey: acc.pubkey.to_string(),
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect();

    // Build the final response data
    let response_data = InstructionResponse {
        program_id: instruction.program_id.to_string(),
        accounts,
        instruction_data: general_purpose::STANDARD.encode(&instruction.data),
    };

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        data: Some(response_data),
        error: None,
    })
}

// --- 4. Health Check Endpoint ---

/// Handles GET /health
/// Helps Render detect the open port.
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

// --- Main Server Setup ---

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let bind_address = format!("{}:{}", host, port);

    println!("🚀 Starting Solana HTTP server at http://{}", bind_address);

    match HttpServer::new(|| {
        App::new()
            .route("/keypair", web::post().to(generate_keypair))
            .route("/token/create", web::post().to(create_token))
            .route("/token/mint", web::post().to(mint_token))
            .route("/health", web::get().to(health))
    })
    .bind(&bind_address) {
        Ok(server) => {
            println!("Successfully bound to {}", bind_address);
            server.run().await?;
            println!("Server is running");
            Ok(())
        }
        Err(e) => {
            println!("Failed to bind to {}: {:?}", bind_address, e);
            Err(e)
        }
    }
}