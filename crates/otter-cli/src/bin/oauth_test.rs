use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope,
    TokenResponse, TokenUrl, basic::BasicClient, reqwest::async_http_client,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const REDIRECT_URI: &str = "http://localhost:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 OAuth2 PKCE Test Tool");
    println!("========================\n");

    // Load Client ID and Secret from env variables
    let client_id = std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| {
            // Public Client ID - safe to distribute
            "251946123352-bp2baikvt4817semo2d541dd2ffov6lk.apps.googleusercontent.com".to_string()
        });
    
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .map_err(|_| "❌ GOOGLE_CLIENT_SECRET non configurato. Copia .env.example in .env e aggiungi il secret.")?;

    println!("📋 Configurazione:");
    println!("  Client ID: {}", client_id);
    println!("  Client Secret: {}...{}", &client_secret[..10], &client_secret[client_secret.len()-4..]);
    println!("  Redirect URI: {}", REDIRECT_URI);
    println!("  PKCE: Enabled\n");

    // Create OAuth2 client with ClientSecret (required for installed apps)
    let oauth_client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
        Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?),
    )
    .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string())?);

    // Step 1: Generate PKCE challenge
    println!("📝 Step 1: Generazione PKCE challenge...");
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    println!("✓ PKCE challenge generato\n");

    // Step 2: Generate authorization URL
    println!("🔗 Step 2: Generazione authorization URL...");
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/drive.file".to_string()))
        .add_scope(Scope::new("openid".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("✓ Authorization URL generato\n");
    println!("📲 Apri questo URL nel browser:");
    println!("  {}\n", auth_url.as_str());

    // Step 3: Start local server to receive callback
    println!("🔄 Step 3: In attesa di callback da Google...");
    println!("  (Dopo il login, Google ti reindirizza a localhost:8080)\n");

    let listener = TcpListener::bind("127.0.0.1:8080")
        .map_err(|e| format!("Errore bind porta 8080: {}. È già in uso?", e))?;

    let code = {
        let (mut stream, _) = listener.accept()?;
        let code;
        let state;

        {
            let mut reader = BufReader::new(&stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line)?;

            let redirect_url = request_line
                .split_whitespace()
                .nth(1)
                .ok_or("Richiesta HTTP invalida")?;
            
            let url = url::Url::parse(&format!("http://localhost{}", redirect_url))?;

            let code_pair = url
                .query_pairs()
                .find(|(key, _)| key == "code")
                .ok_or("Authorization code non trovato")?;

            let state_pair = url
                .query_pairs()
                .find(|(key, _)| key == "state")
                .ok_or("CSRF token non trovato")?;

            code = AuthorizationCode::new(code_pair.1.into_owned());
            state = CsrfToken::new(state_pair.1.into_owned());

            println!("✓ Callback ricevuto da Google!");
            println!("  Authorization code: {}\n", code.secret());
        }

        // Verify CSRF token
        if state.secret() != csrf_token.secret() {
            return Err("❌ CSRF token non valido".into());
        }
        println!("✓ CSRF token valido\n");

        // Send success response to browser
        let response = "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\n\r\n<html><body><h1>✅ Autenticazione completata!</h1><p>Torna al terminale.</p></body></html>";
        stream.write_all(response.as_bytes())?;

        code
    };

    // Step 4: Exchange authorization code for token
    println!("🔑 Step 4: Scambio authorization code con token...");
    
    let token_result = oauth_client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await;

    match token_result {
        Ok(token) => {
            println!("✓ Token ricevuto con successo!\n");
            println!("📊 Token Response:");
            println!("  Access Token: {}", token.access_token().secret());
            println!("  Token Type: Bearer");
            if let Some(expiry) = token.expires_in() {
                println!("  Expires In: {:?}", expiry);
            }
            if let Some(refresh) = token.refresh_token() {
                println!("  Refresh Token: {}", refresh.secret());
            }
            println!();

            // Step 5: Get user info
            println!("👤 Step 5: Recupero informazioni utente...");
            let response = reqwest::Client::new()
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .bearer_auth(token.access_token().secret())
                .send()
                .await?;

            let user_info: serde_json::Value = response.json().await?;

            println!("✓ Informazioni utente ricevute\n");
            println!("👥 User Info:");
            println!("  Email: {}", user_info["email"].as_str().unwrap_or("N/A"));
            println!("  Name: {}", user_info["name"].as_str().unwrap_or("N/A"));
            println!("  ID: {}", user_info["id"].as_str().unwrap_or("N/A"));
            println!();

            println!("✅ OAuth Test completato con SUCCESSO!\n");
            Ok(())
        }
        Err(e) => {
            println!("❌ Errore scambio token:\n");
            println!("  Dettagli: {:?}\n", e);
            println!("🔍 Possibili cause:");
            println!("  1. Client ID non configurato correttamente");
            println!("  2. Redirect URI non registrato in Google Console");
            println!("  3. Authorization code scaduto (valido 10 minuti)");
            println!("  4. PKCE challenge non corrisponde\n");
            Err(format!("Errore: {:?}", e).into())
        }
    }
}
