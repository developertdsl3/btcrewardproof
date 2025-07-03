use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use alloy_sol_types::SolType;
use reqwest;
use serde::{Deserialize, Serialize};
use sp1_sdk::{include_elf, ProverClient, SP1Stdin, setup_logger, HashableKey};
use std::error::Error;
use hex;
use fibonacci_lib::{ PublicValuesIcr, PublicValuesLiquidation, PublicValuesLtv, PublicValuesAddress,PublicValuesCatalyst};
use tokio::task;
use anyhow::Result;
use reqwest::get;

// Program binary
#[allow(unused_variables, unused_imports, dead_code)]
pub const DEFI_ELF: &[u8] = include_elf!("fibonacci-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const ICR_ELF: &[u8] = include_elf!("icr-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const LIQUIDATION_ELF: &[u8] = include_elf!("liquid-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const REAL_TIME_LTV_ELF: &[u8] = include_elf!("Real_time_ltv-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const USER_ADDRESS_ELF: &[u8] = include_elf!("user_balance-program");
#[allow(unused_variables, unused_imports, dead_code)]
pub const Catalysis_ELF: &[u8] = include_elf!("catalysis-program");

// Minimum ICR required (150%)
const MIN_ICR: u32 = 150;

// === Data structures ===
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponse {
    icr: u32,
    collateral_value_usd: u32,
    liquidation_threshold: u32,
    real_time_ltv: u32,
    btc_price_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}




// #[derive(Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct UtxoStructure {
//     txid: String,
//     vout: u32,
//     amount: u64,
//     script_pubkey: String,
   
// }


#[derive(Debug, Deserialize)]
struct Utxo {
    value: u64, // in satoshis
}
pub async fn fetch_user_btc_balance(user_address: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let url = format!("https://mempool.space/api/address/{}/utxo", user_address);
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch UTXO: {}", response.status()).into());
    }

    let utxos: Vec<Utxo> = response.json().await?;

    let total_sats: u64 = utxos.iter().map(|u| u.value).sum();
    let total_btc = total_sats as f64 / 100_000_000.0;

    Ok(total_btc)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataLiquidationResponse {
    liquidation_threshold: u32,
    vkey: String,
    public_values: String,
    proof: String,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalysisResponse {
    user_address: String,
    reward_amount: u32,
    vkey: String,
    public_values: String,
    proof: String,
    
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataLiquidation {
    collateral_amount: u32,
    min_icr: u32,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCatalysis {
    user_address: String,
    reward_amount: u32,
    proof_system: String,
    
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseIcr {
    icr: u32,
    collateral_value_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseUserBalance {
    user_address: String,
    btc_price_usd: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponseLtv {
    real_time_ltv: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestIcr {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofRequestLtv {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAddress {
    user_address: String,
    proof_system: String,
    
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcrResponse {
    icr: u32,
    vkey: String,
    public_values: String,
    proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateProofRequest {
    collateral_amount: u32,
    debt_amount: u32,
    proof_system: String,
}

#[derive(Debug, Deserialize)]
struct BtcPriceResponse {
    bitcoin: BtcPrice,
}

#[derive(Debug, Deserialize)]
struct BtcPrice {
    usd: f64,
}



#[post("/generate-proof-icr")]
async fn prove_icr_final(req: web::Json<ProofRequestIcr>) -> impl Responder {
    println!("Received ICR proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    // Wrap blocking operations in spawn_blocking
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(ICR_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.collateral_amount);
        stdin.write(&req.debt_amount);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesIcr::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = ProofResponseIcr {
        icr: public_values.icr,
        collateral_value_usd: public_values.collateral_amount,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-liquidation")]
async fn prove_liquidation(req: web::Json<UserDataLiquidation>) -> impl Responder {
    println!("Received liquidation proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    // Wrap blocking operations in spawn_blocking
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(LIQUIDATION_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.collateral_amount);
        stdin.write(&req.min_icr);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesLiquidation::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = UserDataLiquidationResponse {
        liquidation_threshold: public_values.liquidation_threshold,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-ltv")]
async fn prove_ltv(req: web::Json<ProofRequestLtv>) -> impl Responder {
    println!("Received LTV proof generation request: {:?}", req);

    let btc_price = match fetch_btc_price().await {
        Ok(price) => price,
        Err(e) => {
            eprintln!("Failed to fetch BTC price: {:?}", e);
            return HttpResponse::InternalServerError().body("BTC price fetch failed");
        }
    };
    let btc_price_usd = btc_price as u32;

    // Wrap blocking operations in spawn_blocking
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(REAL_TIME_LTV_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&req.debt_amount);
        stdin.write(&req.collateral_amount);
        stdin.write(&btc_price_usd);

        let proof_result = match req.proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesLtv::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = ProofResponseLtv {
        real_time_ltv: public_values.real_time_ltv,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}
#[post("/generate-proof-user-balance")]
async fn prove_userBalance(req: web::Json<UserAddress>) -> impl Responder {
    println!("Received User Balance: {:?}", req);

    let user_address = req.user_address.clone(); // Clone to own the value
    let proof_system = req.proof_system.clone(); // Clone to own the value
    let btc_price_usd = match fetch_user_btc_balance(&user_address).await { // Add .await
        Ok(price) => price as u32,
        Err(e) => {
            eprintln!("Failed to fetch user BTC balance: {:?}", e);
            return HttpResponse::InternalServerError().body("User BTC balance fetch failed");
        }
    };

    // Wrap blocking operations in spawn_blocking
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(USER_ADDRESS_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&user_address.as_bytes());
        stdin.write(&btc_price_usd);

        let proof_result = match proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesAddress::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = ProofResponseUserBalance {
        user_address: public_values.user_address.to_string(),
        btc_price_usd,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}

#[post("/generate-proof-catalysis")]
async fn prove_catalysis(req: web::Json<UserCatalysis>) -> impl Responder {
    println!("Received Catalysis Request: {:?}", req);

    let user_address = req.user_address.clone(); // Clone to own the value
    let reward_amount = req.reward_amount;
    let proof_system = req.proof_system.clone(); // Clone to own the value

    // Wrap blocking operations in spawn_blocking
    let proof_result = task::spawn_blocking(move || {
        let client = ProverClient::from_env();
        let (pk, vk) = client.setup(Catalysis_ELF);

        let mut stdin = SP1Stdin::new();
        stdin.write(&user_address.as_bytes());
        stdin.write(&reward_amount); // Changed from .input to .write

        let proof_result = match proof_system.as_str() {
            "plonk" => client.prove(&pk, &stdin).plonk().run(),
            "groth16" => client.prove(&pk, &stdin).groth16().run(),
            _ => return Err(anyhow::anyhow!("Invalid proof system")),
        };
        Ok(proof_result.map(|proof| (proof, vk))?)
    })
    .await;

    let (proof, vk) = match proof_result {
        Ok(Ok((proof, vk))) => (proof, vk),
        Ok(Err(e)) => {
            eprintln!("Proof generation failed: {:?}", e);
            return HttpResponse::InternalServerError().body(format!("Proof generation failed: {}", e));
        }
        Err(e) => {
            eprintln!("Proof generation task failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Proof generation task failed");
        }
    };

    let public_bytes = proof.public_values.as_slice();
    eprintln!("Public values bytes: {:?}", hex::encode(public_bytes));
    let public_values = match PublicValuesCatalyst::abi_decode(public_bytes) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Decoding public values failed: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to decode public values");
        }
    };

    let response = CatalysisResponse {
        user_address: public_values.user_address.to_string(),
        reward_amount: public_values.reward_amount,
        vkey: vk.bytes32(),
        public_values: format!("0x{}", hex::encode(public_bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    HttpResponse::Ok().json(response)
}









// === Helper functions ===
async fn fetch_btc_price() -> Result<f64, Box<dyn Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let resp = reqwest::get(url).await?;
    let text = resp.text().await?;
    println!("CoinGecko response: {}", text);
    let json: BtcPriceResponse = serde_json::from_str(&text)?;
    if json.bitcoin.usd <= 0.0 {
        return Err("BTC price is zero or negative".into());
    }
    Ok(json.bitcoin.usd)
}



#[tokio::main]
async fn main() -> std::io::Result<()> {
    setup_logger();
    println!("Starting DeFi SP1 proof server on http://localhost:3002");

    HttpServer::new(|| {
        App::new()
            .service(prove_icr_final)
            .service(prove_liquidation)
            .service(prove_ltv)
            .service(prove_userBalance)
            .service(prove_catalysis)
    })
    .workers(4)
    .bind(("0.0.0.0", 3002))?
    .run()
    .await
}
