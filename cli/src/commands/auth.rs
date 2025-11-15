use crate::config::{AppConfig, AnthropicOAuth};
use clap::Subcommand;
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
const OAUTH_TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const REDIRECT_URI: &str = "https://console.anthropic.com/oauth/code/callback";
const SCOPES: &str = "org:create_api_key user:profile user:inference";

#[derive(Subcommand, PartialEq)]
pub enum AuthCommands {
    /// Login to Anthropic/Claude Code with OAuth
    Login {
        /// Provider to login to
        #[arg(default_value = "anthropic")]
        provider: String,
    },
    /// Logout from Anthropic/Claude Code
    Logout {
        /// Provider to logout from
        #[arg(default_value = "anthropic")]
        provider: String,
    },
}

/// Generate PKCE code verifier and challenge
fn generate_pkce() -> (String, String) {
    // Generate random 32-byte verifier
    let mut rng = rand::rng();
    let random_bytes: [u8; 32] = rng.random();
    let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

    // Generate SHA256 challenge
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge_bytes = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    (verifier, challenge)
}

/// Exchange authorization code for tokens
async fn exchange_code_for_tokens(
    code: &str,
    state: &str,
    verifier: &str,
) -> Result<(String, String, u64), String> {
    let client = reqwest::Client::new();

    let response = client
        .post(OAUTH_TOKEN_URL)
        .json(&serde_json::json!({
            "code": code,
            "state": state,
            "grant_type": "authorization_code",
            "client_id": CLIENT_ID,
            "redirect_uri": REDIRECT_URI,
            "code_verifier": verifier,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code: {}", e))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Token exchange failed: {}", error_text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let refresh_token = json["refresh_token"]
        .as_str()
        .ok_or("Missing refresh_token")?
        .to_string();

    let access_token = json["access_token"]
        .as_str()
        .ok_or("Missing access_token")?
        .to_string();

    let expires_in = json["expires_in"].as_u64().unwrap_or(3600);

    // Calculate expiry time in milliseconds
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to get current time: {}", e))?
        .as_millis() as u64;

    let expires = now + (expires_in * 1000);

    Ok((refresh_token, access_token, expires))
}

pub async fn handle_auth_command(command: AuthCommands, config: &mut AppConfig) -> Result<(), String> {
    match command {
        AuthCommands::Login { provider } => {
            if provider != "anthropic" {
                return Err(format!("Unsupported provider: {}. Currently only 'anthropic' is supported.", provider));
            }

            println!("\n🔐 Anthropic/Claude Code OAuth Login\n");
            println!("This will open your browser to authenticate with Claude Pro/Max.\n");

            // Generate PKCE
            let (verifier, challenge) = generate_pkce();

            // Build authorization URL
            let auth_url = format!(
                "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
                OAUTH_AUTHORIZE_URL,
                CLIENT_ID,
                urlencoding::encode(REDIRECT_URI),
                urlencoding::encode(SCOPES),
                challenge,
                verifier
            );

            println!("Opening browser for authentication...");
            println!("\nIf the browser doesn't open automatically, visit this URL:");
            println!("{}\n", auth_url);

            // Try to open browser
            if let Err(e) = open::that(&auth_url) {
                eprintln!("Failed to open browser: {}", e);
            }

            println!("After authorizing, you will be redirected to a page showing an authorization code.");
            println!("The code will be in the format: <code>#<state>");
            print!("\nPaste the authorization code here: ");
            use std::io::{self, Write};
            io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| format!("Failed to read input: {}", e))?;

            let full_code = input.trim();

            // Split code and state
            let parts: Vec<&str> = full_code.split('#').collect();
            if parts.len() != 2 {
                return Err("Invalid authorization code format. Expected: <code>#<state>".to_string());
            }

            let code = parts[0];
            let state = parts[1];

            println!("\nExchanging authorization code for tokens...");

            // Exchange code for tokens
            let (refresh_token, access_token, expires) =
                exchange_code_for_tokens(code, state, &verifier).await?;

            // Update config
            config.anthropic_oauth = Some(AnthropicOAuth {
                refresh_token,
                access_token,
                expires,
            });
            config.provider = Some("anthropic".to_string());

            // Save config
            config.save()?;

            println!("\n✅ Successfully logged in to Anthropic/Claude Code!");
            println!("You can now use stakpak with your Claude Pro/Max subscription.\n");

            Ok(())
        }

        AuthCommands::Logout { provider } => {
            if provider != "anthropic" {
                return Err(format!("Unsupported provider: {}. Currently only 'anthropic' is supported.", provider));
            }

            if config.anthropic_oauth.is_none() {
                println!("Not logged in to Anthropic/Claude Code.");
                return Ok(());
            }

            config.anthropic_oauth = None;

            // If there's no stakpak API key, clear the provider too
            if config.api_key.is_none() {
                config.provider = None;
            }

            config.save()?;

            println!("✅ Successfully logged out from Anthropic/Claude Code.");

            Ok(())
        }
    }
}
