use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use candid::{CandidType, Principal};
use ic_agent::Identity;
use ic_agent::{agent::http_transport::ReqwestTransport, identity, Agent};
use indicatif::ProgressBar;
use reqwest::Client;
use serde::Deserialize;
use serde_bytes::ByteBuf;
use std::{io::Write, time::Duration};
use tokio::time::Instant;

const CANISTER_METHOD: &str = "write";
const NUMBER_OF_REQUESTS: u64 = 100_000;

const API_BN_URL: &str = "https://testic0.app";

const CHALLENGE_SOLVED: bool = false;
const COUNTER_EFFECTIVE_CANISTER_ID: &str = "3muos-6yaaa-aaaaa-qaaua-cai"; // counter canister
const II_CANISTER_ID: &str = "rdmx6-jaaaa-aaaaa-aaadq-cai"; // internet identity canister

use ring::rand::SystemRandom;
use ring::signature::Ed25519KeyPair;

#[tokio::main]
async fn main() {
    benchmark_counter_canister().await;
}

async fn benchmark_counter_canister() {
    // let v2_agent = {
    //     let transport = Client::builder()
    //         .use_rustls_tls()
    //         .timeout(Duration::from_secs(360))
    //         .build()
    //         .expect("Could not create HTTP client.");

    //     let v2_reqwest_transport =
    //         ReqwestTransport::create_with_client(API_BN_URL, transport).unwrap();

    //     Agent::builder()
    //         .with_transport(v2_reqwest_transport)
    //         .build()
    //         .unwrap()
    // };

    let v3_agent = {
        let transport = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(360))
            .build()
            .expect("Could not create HTTP client.");

        let reqwest_transport = ReqwestTransport::create_with_client(API_BN_URL, transport)
            .unwrap()
            .with_use_call_v3_endpoint();

        Agent::builder()
            .with_transport(reqwest_transport)
            .build()
            .unwrap()
    };

    let effective_canister_id = Principal::from_text(COUNTER_EFFECTIVE_CANISTER_ID).unwrap();

    // create a file and write durations to it
    // let mut v2_file = std::fs::File::create("v2_latencies.txt").unwrap();
    let mut v3_file = std::fs::File::create("v3_latencies.txt").unwrap();

    let bar = ProgressBar::new(NUMBER_OF_REQUESTS);

    loop {
        for (agent, file) in vec![
            (&v3_agent, &mut v3_file),
            // (&v2_agent, &mut v2_file)
        ] {
            loop {
                let start = Instant::now();
                match agent
                    .update(&effective_canister_id, CANISTER_METHOD)
                    .call_and_wait()
                    .await
                {
                    Ok(_) => {
                        let elapsed = format!("{}\n", start.elapsed().as_millis());
                        file.write_all(elapsed.as_bytes()).unwrap();
                        break;
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                    }
                }
            }
        }

        bar.inc(1);
    }
}

async fn benchmark_internet_identity_canister() {
    let ii_canister_principal = candid::Principal::from_text(II_CANISTER_ID).unwrap();

    let (agent, pub_key) = {
        let transport = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(360))
            .build()
            .expect("Could not create HTTP client.");

        let reqwest_transport = ReqwestTransport::create_with_client(API_BN_URL, transport)
            .unwrap()
            .with_use_call_v3_endpoint();

        let identity = {
            // check if the file exists
            if !std::path::Path::new("pkcs8_bytes").exists() {
                // generate a new key pair
                let rng = SystemRandom::new();
                let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();

                // store the pkcs8_bytes in a file
                std::fs::write("pkcs8_bytes", pkcs8_bytes.as_ref()).unwrap();
            }

            let pkcs8_bytes = std::fs::read("pkcs8_bytes").unwrap();
            let key = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref()).unwrap();

            identity::BasicIdentity::from_key_pair(key)
        };

        let pub_key = identity.public_key().unwrap();

        (
            Agent::builder()
                .with_transport(reqwest_transport)
                .with_identity(identity)
                .build()
                .unwrap(),
            pub_key,
        )
    };

    if !std::path::Path::new("user_number").exists() {
        let response = agent
            .update(&ii_canister_principal, "create_challenge")
            .with_arg(candid::encode_one(()).unwrap())
            .call_and_wait()
            .await
            .unwrap();

        let challenge: Challenge = candid::decode_one(&response).unwrap();
        // display the challenge to the user
        let challenge_png = BASE64_STANDARD.decode(challenge.png_base64).unwrap();
        std::fs::write("challenge.png", challenge_png).unwrap();

        // open the image

        // wait for the user to solve the challenge
        let mut input = String::new();
        println!("Enter the characters in the image: ");
        std::io::stdin().read_line(&mut input).unwrap();

        let device = DeviceData {
            pubkey: ByteBuf::from(pub_key.clone()),
            alias: "test key".to_string(),
            credential_id: None,
            purpose: Purpose::Authentication,
            key_type: KeyType::Unknown,
            protection: DeviceProtection::Unprotected,
        };

        let challenge_attempt = ChallengeAttempt {
            chars: input.trim().to_string(),
            key: challenge.challenge_key,
        };

        let data: Vec<u8> = agent
            .update(&ii_canister_principal, "register")
            .with_arg(candid::encode_args((device, challenge_attempt)).unwrap())
            .call_and_wait()
            .await
            .unwrap();

        let register_response: RegisterResponse = candid::decode_one(&data).unwrap();

        let user_number = match register_response {
            RegisterResponse::Registered { user_number } => user_number,
            x => panic!("Could not register user: {:?}", x),
        };

        // store user nubmer to file
        std::fs::write("user_number", user_number.to_string()).unwrap();
    }

    let user_number = std::fs::read_to_string("user_number")
        .unwrap()
        .parse::<u64>()
        .unwrap();

    // Don't overwrite the file if it already exists
    let mut latency_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("ii_latencies_tokamak_complete.txt")
        .unwrap();

    let bar = ProgressBar::new(NUMBER_OF_REQUESTS);
    let mut i = 0_u32;
    loop {
        let frontend_hostname = format!("https://nns.ic0.app");

        let duration = create_delegation(
            &agent,
            pub_key.clone(),
            ii_canister_principal,
            frontend_hostname.clone(),
            user_number,
        )
        .await;

        let Some(duration) = duration else {
            continue;
        };

        let elapsed = format!("{}\n", duration.as_millis());
        latency_file.write_all(elapsed.as_bytes()).unwrap();

        bar.inc(1);
        i += 1;
    }
}

pub async fn create_delegation(
    agent: &Agent,
    delegation_pubkey: Vec<u8>,
    ii_canister_id: Principal,
    canister_url: String,
    user_id: u64,
) -> Option<Duration> {
    let start = Instant::now();

    let data: Vec<u8> = agent
        .update(&ii_canister_id, "prepare_delegation")
        .with_arg(
            candid::encode_args((
                user_id,
                canister_url.clone(),
                ByteBuf::from(delegation_pubkey.clone()),
                None::<u64>,
            ))
            .unwrap(),
        )
        .call_and_wait()
        .await
        .inspect_err(|e| eprintln!("Error while preparing delegation: {:?}", e))
        .ok()?;

    let (ii_derived_public_key, timestamp): (UserKey, Timestamp) =
        candid::decode_args(&data).unwrap();

    let data: Vec<u8> = agent
        .query(&ii_canister_id, "get_delegation")
        .with_arg(
            candid::encode_args((
                user_id,
                canister_url,
                ByteBuf::from(delegation_pubkey.clone()),
                timestamp,
            ))
            .unwrap(),
        )
        .call()
        .await
        .ok()?;

    let delegation_response: GetDelegationResponse = candid::decode_one(&data).unwrap();
    match delegation_response {
        GetDelegationResponse::SignedDelegation(delegation) => delegation,
        GetDelegationResponse::NoSuchDelegation => {
            panic!("unexpected get_delegation result: NoSuchDelegation")
        }
    };

    Some(start.elapsed())
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct Delegation {
    pub pubkey: PublicKey,
    pub expiration: Timestamp,
    pub targets: Option<Vec<Principal>>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct SignedDelegationCustom {
    pub delegation: Delegation,
    pub signature: Signature,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
enum GetDelegationResponse {
    #[serde(rename = "signed_delegation")]
    SignedDelegation(SignedDelegationCustom),
    #[serde(rename = "no_such_delegation")]
    NoSuchDelegation,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct Challenge {
    pub png_base64: String,
    pub challenge_key: String,
}

// The user's attempt
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct ChallengeAttempt {
    pub chars: String,
    pub key: String,
}

pub type AnchorNumber = u64;

pub type CredentialId = ByteBuf;
pub type PublicKey = ByteBuf;
pub type DeviceKey = PublicKey;
pub type UserKey = PublicKey;
// in nanos since epoch
pub type Timestamp = u64;
type Signature = ByteBuf;

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
struct DeviceData {
    pub pubkey: DeviceKey,
    pub alias: String,
    pub credential_id: Option<CredentialId>,
    pub purpose: Purpose,
    pub key_type: KeyType,
    pub protection: DeviceProtection,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
enum KeyType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "platform")]
    Platform,
    #[serde(rename = "cross_platform")]
    CrossPlatform,
    #[serde(rename = "seed_phrase")]
    SeedPhrase,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub enum DeviceProtection {
    #[serde(rename = "protected")]
    Protected,
    #[serde(rename = "unprotected")]
    Unprotected,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
pub enum Purpose {
    #[serde(rename = "recovery")]
    Recovery,
    #[serde(rename = "authentication")]
    Authentication,
}

#[derive(Clone, Eq, PartialEq, Debug, CandidType, Deserialize)]
enum RegisterResponse {
    #[serde(rename = "registered")]
    Registered { user_number: AnchorNumber },
    #[serde(rename = "canister_full")]
    CanisterFull,
    #[serde(rename = "bad_challenge")]
    BadChallenge,
}
