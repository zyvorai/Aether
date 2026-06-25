// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
//
// licensegen — Zyvor-internal tool to generate signed Zeus OS license files.
//
// Usage:
//   licensegen --customer "ABC Pvt Ltd" --customer-id abc-pvt-ltd \
//     --allowed-nodes 25 --allowed-clusters 3 \
//     --valid-from 2026-06-25 --valid-until 2027-06-24 \
//     --private-key /path/to/zyvor_private.pem \
//     --out license.zyvor

use anyhow::{Context, Result};
use base64::Engine;
use clap::Parser;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{Signer, SignatureEncoding};
use rsa::RsaPrivateKey;
use sha2::Sha256;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Generate a signed Zeus OS (.zyvor) license file")]
struct Args {
    /// Customer display name (e.g. "ABC Pvt Ltd")
    #[arg(long)]
    customer: String,

    /// Customer ID slug (e.g. "abc-pvt-ltd")
    #[arg(long)]
    customer_id: String,

    /// Number of managed Kubernetes nodes licensed
    #[arg(long)]
    allowed_nodes: u32,

    /// Number of Kubernetes clusters licensed
    #[arg(long)]
    allowed_clusters: u32,

    /// License start date in YYYY-MM-DD format
    #[arg(long)]
    valid_from: String,

    /// License expiry date in YYYY-MM-DD format
    #[arg(long)]
    valid_until: String,

    /// Path to the Zyvor RSA private key (PKCS#8 PEM)
    #[arg(long)]
    private_key: PathBuf,

    /// Output path for the .zyvor license file
    #[arg(long, default_value = "license.zyvor")]
    out: PathBuf,

    /// Override the generated license ID (e.g. ZV-ZEUS-2026-0001)
    #[arg(long)]
    license_id: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let pem = std::fs::read_to_string(&args.private_key)
        .with_context(|| format!("cannot read private key {:?}", args.private_key))?;

    let private_key =
        RsaPrivateKey::from_pkcs8_pem(&pem).context("invalid PKCS#8 RSA private key PEM")?;

    let signing_key = SigningKey::<Sha256>::new(private_key);

    // Use provided license ID or generate one from the counter file
    let license_id = args.license_id.unwrap_or_else(|| {
        let year = chrono::Utc::now().format("%Y");
        format!("ZV-ZEUS-{}-{:04}", year, next_sequence())
    });

    let issued_at = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let claims = serde_json::json!({
        "license_id": license_id,
        "product": "zeus-os",
        "customer": args.customer,
        "customer_id": args.customer_id,
        "allowed_nodes": args.allowed_nodes,
        "allowed_clusters": args.allowed_clusters,
        "valid_from": args.valid_from,
        "valid_until": args.valid_until,
        "issued_at": issued_at,
        "license_version": 1,
    });

    let payload_bytes = serde_json::to_vec(&claims).context("failed to serialize claims")?;
    let payload_b64 =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_bytes);

    let signature = signing_key.sign(&payload_bytes);
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(signature.to_bytes().as_ref());

    let envelope = serde_json::json!({
        "format": "zyvor-v1",
        "payload": payload_b64,
        "signature": sig_b64,
    });

    let json = serde_json::to_string_pretty(&envelope).context("failed to serialize envelope")?;
    std::fs::write(&args.out, &json)
        .with_context(|| format!("cannot write license to {:?}", args.out))?;

    println!("License written to {}", args.out.display());
    println!("  License ID  : {}", claims["license_id"]);
    println!("  Customer    : {}", args.customer);
    println!("  Nodes       : {}", args.allowed_nodes);
    println!("  Clusters    : {}", args.allowed_clusters);
    println!("  Valid from  : {}", args.valid_from);
    println!("  Valid until : {}", args.valid_until);

    Ok(())
}

/// Read/increment a simple counter in ~/.zyvor-licensegen-counter for unique IDs.
fn next_sequence() -> u32 {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zyvor-licensegen-counter");
    let n: u32 = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
        + 1;
    let _ = std::fs::write(&path, n.to_string());
    n
}
