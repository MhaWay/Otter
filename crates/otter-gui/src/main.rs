#![windows_subsystem = "windows"]

use iced::{
    widget::{Column, Container, Row, Scrollable, Text, Button, Space, TextInput, text, button, Svg},
    Element, Length, Font, Alignment, Task, Border,
    Background, Color, Shadow,
    time, Subscription,
};
use iced::widget::svg;
use iced::widget::button::Status;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
    basic::BasicClient, reqwest::async_http_client,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

// OAuth credentials loaded from environment variables
// Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables
const REDIRECT_URI: &str = "http://localhost:8080";

const ROBOTO_FONT: Font = Font::with_name("Roboto");

// Source - https://stackoverflow.com/a/79782372
// Posted by Péter Szilvási, modified by community. See post 'Timeline' for change history
// Retrieved 2026-02-17, License - CC BY-SA 4.0

fn main() -> iced::Result {
    iced::application(
        || GuiApp::new(),
        GuiApp::update,
        GuiApp::view
    )
    .title("Otter - Privacy-Focused Chat")
    .window(iced::window::Settings {
        exit_on_close_request: true,
        ..Default::default()
    })
    .font(include_bytes!("../fonts/Roboto-Regular.woff2").as_slice())
    .subscription(GuiApp::subscription)
    .run()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Identity {
    pub id: String,
    pub peer_id: String,
    pub nickname: String,
    pub google_email: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
struct GoogleAuthData {
    pub email: String,
    pub name: String,
    pub picture_url: Option<String>,
    pub access_token: String,
}

#[derive(Clone, Debug)]
enum Message {
    TryLogin,
    LoginGoogleAuthSuccess(GoogleAuthData),
    LoginIdentityLoaded(Result<Identity, String>),
    StartRegister,
    DisclaimerScrolled(f32),
    DisclaimerAccepted,
    StartGoogleAuth,
    GoogleAuthSuccess(GoogleAuthData),
    GoogleAuthError(String),
    NicknameChanged(String),
    NicknameSubmit,
    IdentitySaved(Result<Identity, String>),
    SpinnerTick,
    BackToHome,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Disclaimer,
    GoogleAuth,
    ChooseNickname,
    Saving,
    MainApp,
}

struct GuiApp {
    current_screen: Screen,
    has_reached_bottom: bool,
    user_identity: Option<Identity>,
    nickname_input: String,
    google_auth_data: Option<GoogleAuthData>,
    auth_error: Option<String>,
    spinner_frame: usize,
}

impl Default for GuiApp {
    fn default() -> Self {
        GuiApp {
            current_screen: Screen::Home,
            has_reached_bottom: false,
            user_identity: None,
            nickname_input: String::new(),
            google_auth_data: None,
            auth_error: None,
            spinner_frame: 0,
        }
    }
}

impl GuiApp {
    fn get_identity_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".otter").join("identity.json")
        } else {
            PathBuf::from(".otter/identity.json")
        }
    }

    fn new() -> (Self, Task<Message>) {
        let mut app = GuiApp::default();
        if let Some(identity) = GuiApp::load_identity() {
            app.user_identity = Some(identity);
            app.current_screen = Screen::MainApp;
        } else {
            app.current_screen = Screen::Home;
        }
        (app, Task::none())
    }

    fn load_identity() -> Option<Identity> {
        let path = Self::get_identity_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(identity) = serde_json::from_str::<Identity>(&content) {
                    return Some(identity);
                }
            }
        }
        None
    }

    fn save_identity(identity: &Identity) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_identity_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(identity)?;
        fs::write(&path, json)?;
        Ok(())
    }

    fn delete_identity() -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_identity_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn create_new_identity(nickname: &str, google_email: &str) -> Identity {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        
        let id = uuid::Uuid::new_v4().to_string();
        let peer_id = format!("peer_{}", uuid::Uuid::new_v4().to_string());
        
        Identity {
            id,
            peer_id,
            nickname: nickname.to_string(),
            google_email: google_email.to_string(),
            public_key: "pk_placeholder".to_string(),
            created_at: timestamp,
        }
    }

    async fn load_identity_from_drive(access_token: String) -> Result<Identity, String> {
        let client = reqwest::Client::new();
        
        // Search for .otter folder
        let search_folder = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", "name='.otter' and mimeType='application/vnd.google-apps.folder' and trashed=false")])
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca cartella: {}", e))?;
        
        let folder_result: serde_json::Value = search_folder.json().await
            .map_err(|e| format!("Errore parsing: {}", e))?;
        
        let folder_id = if let Some(files) = folder_result["files"].as_array() {
            if let Some(folder) = files.first() {
                folder["id"].as_str().ok_or("ID cartella mancante")?.to_string()
            } else {
                return Err("Identità non trovata. Devi registrarti prima.".to_string());
            }
        } else {
            return Err("Identità non trovata. Devi registrarti prima.".to_string());
        };
        
        // Search for identity.json in .otter folder
        let search_file = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", format!("name='identity.json' and '{}' in parents and trashed=false", folder_id).as_str())])
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca file: {}", e))?;
        
        let file_result: serde_json::Value = search_file.json().await
            .map_err(|e| format!("Errore parsing: {}", e))?;
        
        let file_id = if let Some(files) = file_result["files"].as_array() {
            if let Some(file) = files.first() {
                file["id"].as_str().ok_or("ID file mancante")?.to_string()
            } else {
                return Err("Identità non trovata. Devi registrarti prima.".to_string());
            }
        } else {
            return Err("Identità non trovata. Devi registrarti prima.".to_string());
        };
        
        // Download file content
        let download = client
            .get(format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore download: {}", e))?;
        
        let content = download.text().await
            .map_err(|e| format!("Errore lettura: {}", e))?;
        
        let identity: Identity = serde_json::from_str(&content)
            .map_err(|e| format!("Errore parsing identità: {}", e))?;
        
        // Save locally for offline access
        let _ = GuiApp::save_identity(&identity);
        
        Ok(identity)
    }

    async fn save_identity_to_drive(identity: &Identity, access_token: &str) -> Result<(), String> {
        let client = reqwest::Client::new();
        
        // First, check if .otter folder exists, if not create it
        let search_response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", "name='.otter' and mimeType='application/vnd.google-apps.folder' and trashed=false")])
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca cartella: {}", e))?;
        
        let search_result: serde_json::Value = search_response.json().await
            .map_err(|e| format!("Errore parsing risposta: {}", e))?;
        
        // Get or create folder ID
        let folder_id = if let Some(files) = search_result["files"].as_array() {
            if let Some(folder) = files.first() {
                folder["id"].as_str().unwrap_or("").to_string()
            } else {
                // Create .otter folder
                let folder_metadata = serde_json::json!({
                    "name": ".otter",
                    "mimeType": "application/vnd.google-apps.folder"
                });
                
                let create_response = client
                    .post("https://www.googleapis.com/drive/v3/files")
                    .header("Authorization", format!("Bearer {}", access_token))
                    .header("Content-Type", "application/json")
                    .json(&folder_metadata)
                    .send()
                    .await
                    .map_err(|e| format!("Errore creazione cartella: {}", e))?;
                
                let folder: serde_json::Value = create_response.json().await
                    .map_err(|e| format!("Errore parsing cartella: {}", e))?;
                
                folder["id"].as_str().unwrap_or("").to_string()
            }
        } else {
            return Err("Formato risposta Drive invalido".to_string());
        };
        
        // Create file metadata
        let metadata = serde_json::json!({
            "name": "identity.json",
            "mimeType": "application/json",
            "parents": [folder_id]
        });
        
        let identity_json = serde_json::to_string_pretty(identity)
            .map_err(|e| format!("Errore serializzazione: {}", e))?;
        
        // Create multipart form
        let boundary = "foo_bar_baz";
        let body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n--{boundary}\r\nContent-Type: application/json\r\n\r\n{}\r\n--{boundary}--",
            serde_json::to_string(&metadata).unwrap(),
            identity_json
        );
        
        // Upload to Google Drive
        let response = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Errore upload a Drive: {}", e))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Errore salvataggio Drive: {}", error_text));
        }
        
        Ok(())
    }
}

impl GuiApp {
    fn start_google_auth() -> Task<Message> {
        Task::perform(
            GuiApp::perform_google_auth(),
            |result| match result {
                Ok(data) => Message::GoogleAuthSuccess(data),
                Err(e) => Message::GoogleAuthError(e),
            }
        )
    }

    async fn perform_google_auth() -> Result<GoogleAuthData, String> {
        // Load OAuth credentials from environment variables
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| "GOOGLE_CLIENT_ID environment variable not set".to_string())?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| "GOOGLE_CLIENT_SECRET environment variable not set".to_string())?;
        
        // Create OAuth2 client
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
                .map_err(|e| format!("Invalid auth URL: {}", e))?,
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .map_err(|e| format!("Invalid token URL: {}", e))?)
        )
        .set_redirect_uri(
            RedirectUrl::new(REDIRECT_URI.to_string())
                .map_err(|e| format!("Invalid redirect URL: {}", e))?
        );

        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate authorization URL
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/drive.file".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        // Open browser for user authentication
        if let Err(e) = open::that(auth_url.as_str()) {
            return Err(format!("Impossibile aprire il browser: {}", e));
        }

        // Start local server to receive callback
        let listener = TcpListener::bind("127.0.0.1:8080")
            .map_err(|e| format!("Impossibile avviare server locale: {}", e))?;

        // Wait for the authorization code
        let code = match listener.accept() {
            Ok((mut stream, _)) => {
                let code;
                let state;
                {
                    let mut reader = BufReader::new(&stream);
                    let mut request_line = String::new();
                    reader.read_line(&mut request_line)
                        .map_err(|e| format!("Errore lettura richiesta: {}", e))?;

                    let redirect_url = request_line.split_whitespace().nth(1)
                        .ok_or("Richiesta HTTP invalida")?;
                    let url = url::Url::parse(&format!("http://localhost{}", redirect_url))
                        .map_err(|e| format!("URL parsing error: {}", e))?;

                    let code_pair = url.query_pairs()
                        .find(|(key, _)| key == "code")
                        .ok_or("Codice di autorizzazione non trovato")?;

                    let state_pair = url.query_pairs()
                        .find(|(key, _)| key == "state")
                        .ok_or("CSRF token non trovato")?;

                    code = AuthorizationCode::new(code_pair.1.into_owned());
                    state = CsrfToken::new(state_pair.1.into_owned());
                }

                // Verify CSRF token
                if state.secret() != csrf_token.secret() {
                    return Err("CSRF token non valido".to_string());
                }

                // Send success response to browser
                let response = "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\n\r\n<html><body><h1>✅ Autenticazione completata!</h1><p>Puoi chiudere questa finestra e tornare all'applicazione.</p></body></html>";
                stream.write_all(response.as_bytes())
                    .map_err(|e| format!("Errore invio risposta: {}", e))?;

                code
            },
            Err(e) => return Err(format!("Errore connessione: {}", e)),
        };

        // Exchange code for token
        let token_result = client
            .exchange_code(code)
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| format!("Errore scambio token: {}", e))?;

        // Get user info using access token
        let user_info = reqwest::Client::new()
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(token_result.access_token().secret())
            .send()
            .await
            .map_err(|e| format!("Errore richiesta info utente: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Errore parsing JSON: {}", e))?;

        Ok(GoogleAuthData {
            email: user_info["email"].as_str().unwrap_or("unknown").to_string(),
            name: user_info["name"].as_str().unwrap_or("User").to_string(),
            picture_url: user_info["picture"].as_str().map(|s| s.to_string()),
            access_token: token_result.access_token().secret().to_string(),
        })
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TryLogin => {
                // Try to load from local storage first
                if let Some(identity) = GuiApp::load_identity() {
                    self.user_identity = Some(identity);
                    self.current_screen = Screen::MainApp;
                } else {
                    // No local identity, need to authenticate with Google
                    return Task::perform(
                        GuiApp::perform_google_auth(),
                        |result| match result {
                            Ok(data) => Message::LoginGoogleAuthSuccess(data),
                            Err(e) => Message::GoogleAuthError(e),
                        }
                    );
                }
            }
            Message::LoginGoogleAuthSuccess(google_data) => {
                // After successful Google auth, try to load identity from Drive
                let access_token = google_data.access_token.clone();
                return Task::perform(
                    GuiApp::load_identity_from_drive(access_token),
                    Message::LoginIdentityLoaded
                );
            }
            Message::LoginIdentityLoaded(result) => {
                match result {
                    Ok(identity) => {
                        self.user_identity = Some(identity);
                        self.current_screen = Screen::MainApp;
                        self.auth_error = None;
                    }
                    Err(e) => {
                        self.auth_error = Some(e);
                        self.current_screen = Screen::Home;
                    }
                }
            }
            Message::StartRegister => {
                self.current_screen = Screen::Disclaimer;
                self.has_reached_bottom = false;
                self.nickname_input.clear();
            }
            Message::DisclaimerScrolled(_position) => {
                self.has_reached_bottom = true;
            }
            Message::DisclaimerAccepted => {
                self.current_screen = Screen::GoogleAuth;
                self.nickname_input.clear();
            }
            Message::StartGoogleAuth => {
                // Trigger OAuth2 flow in a task
                return GuiApp::start_google_auth();
            }
            Message::GoogleAuthSuccess(data) => {
                self.google_auth_data = Some(data);
                self.current_screen = Screen::ChooseNickname;
                self.auth_error = None;
            }
            Message::GoogleAuthError(error) => {
                self.auth_error = Some(error);
            }
            Message::NicknameChanged(value) => {
                self.nickname_input = value;
            }
            Message::NicknameSubmit => {
                if !self.nickname_input.trim().is_empty() {
                    if let Some(ref google_data) = self.google_auth_data {
                        let identity = GuiApp::create_new_identity(
                            &self.nickname_input,
                            &google_data.email
                        );
                        
                        // Save locally first
                        if let Err(e) = GuiApp::save_identity(&identity) {
                            self.auth_error = Some(format!("Errore salvataggio locale: {}", e));
                            return Task::none();
                        }
                        
                        // Switch to saving screen
                        self.current_screen = Screen::Saving;
                        
                        // Save to Google Drive asynchronously
                        let identity_clone = identity.clone();
                        let access_token = google_data.access_token.clone();
                        return Task::perform(
                            async move {
                                GuiApp::save_identity_to_drive(&identity_clone, &access_token)
                                    .await
                                    .map(|_| identity_clone)
                            },
                            Message::IdentitySaved
                        );
                    } else {
                        self.auth_error = Some("Dati Google mancanti".to_string());
                    }
                }
            }
            Message::IdentitySaved(result) => {
                match result {
                    Ok(identity) => {
                        self.user_identity = Some(identity);
                        self.current_screen = Screen::MainApp;
                        self.auth_error = None;
                    }
                    Err(e) => {
                        self.auth_error = Some(format!("Errore salvataggio su Drive: {}", e));
                    }
                }
            }
            Message::BackToHome => {
                self.current_screen = Screen::Home;
                self.has_reached_bottom = false;
                self.nickname_input.clear();
                self.auth_error = None;
                self.google_auth_data = None;
            }
            Message::SpinnerTick => {
                self.spinner_frame = self.spinner_frame.wrapping_add(1);
            }
            Message::Logout => {
                let _ = GuiApp::delete_identity();
                self.user_identity = None;
                self.current_screen = Screen::Home;
                self.has_reached_bottom = false;
                self.nickname_input.clear();
                self.google_auth_data = None;
                self.auth_error = None;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.current_screen == Screen::Saving {
            time::every(Duration::from_millis(16))
                .map(|_| Message::SpinnerTick)
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        match self.current_screen {
            Screen::Home => self.view_home(),
            Screen::Disclaimer => self.view_disclaimer(),
            Screen::GoogleAuth => self.view_google_auth(),
            Screen::ChooseNickname => self.view_choose_nickname(),
            Screen::Saving => self.view_saving(),
            Screen::MainApp => self.view_main_app(),
        }
    }

    fn view_home(&self) -> Element<Message> {
        let title = Text::new("🦦 Otter").size(64).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);

        let question = Text::new("Hai già un'identità?").size(22).font(ROBOTO_FONT);

        let error_message = if let Some(ref error) = self.auth_error {
            Text::new(format!("⚠️ {}", error)).size(14).font(ROBOTO_FONT).color(Color::from_rgb(0.8, 0.2, 0.2))
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let login_btn = Button::new(
            Container::new(
                Text::new("📂 Accedi").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.11, 0.42, 0.87), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.15, 0.50, 0.95), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.08, 0.35, 0.75), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::TryLogin);

        let register_btn = Button::new(
            Container::new(
                Text::new("✨ Registrati").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.11, 0.42, 0.87), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.15, 0.50, 0.95), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.08, 0.35, 0.75), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::StartRegister);

        let content = Column::new()
            .push(Space::new().height(Length::Fill))
            .push(title)
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(question)
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(
                Column::new()
                    .push(login_btn)
                    .push(Space::new().height(Length::Fixed(15.0)))
                    .push(register_btn)
                    .push(Space::new().height(Length::Fixed(20.0)))
                    .push(error_message)
                    .align_x(Alignment::Center)
            )
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_disclaimer(&self) -> Element<Message> {
        let title = Text::new("📋 Termini di Servizio e Disclaimer").size(36).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);

        let disclaimer_text = "\
LEGGI ATTENTAMENTE PRIMA DI USARE OTTER\n\n\
1. SOFTWARE SPERIMENTALE\n\
Otter è una piattaforma di comunicazione peer-to-peer decentralizzata sperimentale.\n\n\
2. SALVAMENTO DATI SU GOOGLE\n\
La tua identità (ID, nickname, chiave pubblica) viene salvata nel profilo Google.\n\
Cosa viene salvato:\n\
• La tua identità Otter (ID univoco + nickname)\n\
• La lista dei tuoi contatti\n\
• I MESSAGGI E LE CONVERSAZIONI NON VENGONO SALVATI IN CLOUD per impostazione predefinita\n\n\
I messaggi vengono salvati SOLO se esplicitamente abilitati nelle impostazioni dell'app.\n\n\
3. ELIMINAZIONE DELLA TUA IDENTITÀ\n\
Puoi eliminare la tua identità in tre modi:\n\
• Tramite l'app: vai in Impostazioni → Elimina Identità\n\
• Manualmente: accedi al tuo profilo Google e vai a Google Drive → Elimina la cartella .otter\n\
• Disattivazione dell'account Google: questo eliminerà tutti i dati Otter salvati\n\n\
AVVERTENZA: L'eliminazione della tua identità è permanente e irreversibile.\n\n\
4. PROTEZIONE DEI DATI E PRIVACY\n\
• Le tue chiavi private sono archiviate localmente sul tuo dispositivo\n\
• Non abbiamo accesso alle tue chiavi o ai tuoi messaggi\n\
• Sei il solo responsabile di proteggere il tuo file di identità\n\n\
5. NESSUNA GARANZIA\n\
Otter è fornito senza alcuna garanzia, esplicita o implicita.\n\
Non possiamo recuperare identità perdute o eliminate.\n\n\
6. CONFORMITÀ LEGALE\n\
Sei il solo responsabile di assicurare che il tuo utilizzo di Otter sia conforme a tutte le leggi.\n\n\
ACCETTAZIONE DEI TERMINI\n\
Scorrendo verso il basso e facendo clic su \"Accetto\", riconosci che:\n\
✓ Hai letto e compreso questi termini\n\
✓ Accetti tutti i rischi associati all'utilizzo di Otter";

        let disclaimer_scroll = Scrollable::new(
            Column::new()
                .push(title)
                .push(Space::new().height(Length::Fixed(15.0)))
                .push(Text::new(disclaimer_text).size(14).width(Length::Fill).font(ROBOTO_FONT).shaping(text::Shaping::Advanced))
                .padding(30)
        )
            .height(Length::Fixed(600.0))
            .on_scroll(|_viewport| Message::DisclaimerScrolled(1.0));

        let button_text = if self.has_reached_bottom {
            "✓ Accetto Tutti i Termini"
        } else {
            "↓ Scorri Verso il Basso Per Accettare →"
        };

        let accept_button: Element<Message> = if self.has_reached_bottom {
            Button::new(
                Container::new(
                    Text::new(button_text).size(16).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            )
                .padding([16, 12])
                .width(Length::Fixed(320.0))
                .height(Length::Fixed(56.0))
                .style(|_theme, status| {
                    let (color, shadow) = match status {
                        Status::Active => (Color::from_rgb(0.11, 0.42, 0.87), Shadow::default()),
                        Status::Hovered => (Color::from_rgb(0.15, 0.50, 0.95), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                        Status::Pressed => (Color::from_rgb(0.08, 0.35, 0.75), Shadow::default()),
                        Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                    };
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::WHITE,
                        border: Border::default().rounded(28),
                        shadow,
                        snap: false,
                    }
                })
                .on_press(Message::DisclaimerAccepted)
                .into()
        } else {
            Button::new(
                Container::new(
                    Text::new(button_text).size(16).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            )
                .padding([16, 12])
                .width(Length::Fixed(320.0))
                .height(Length::Fixed(56.0))
                .style(|_theme, status| {
                    let (color, shadow) = match status {
                        Status::Active => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                        Status::Hovered => (Color::from_rgb(0.6, 0.6, 0.6), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                        Status::Pressed => (Color::from_rgb(0.4, 0.4, 0.4), Shadow::default()),
                        Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                    };
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::from_rgb(0.8, 0.8, 0.8),
                        border: Border::default().rounded(28),
                        shadow,
                        snap: false,
                    }
                })
                .into()
        };

        let back_button: Element<Message> = Button::new(
            Container::new(
                Text::new("← Indietro").size(14).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(140.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.6, 0.6, 0.6), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.7, 0.7, 0.7), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::BackToHome)
            .into();

        let button_row = Row::new()
            .push(back_button)
            .push(Space::new().width(Length::Fill))
            .push(accept_button)
            .width(Length::Fill)
            .spacing(15);

        let content = Column::new()
            .push(disclaimer_scroll)
            .push(Space::new().height(Length::Fixed(15.0)))
            .push(button_row)
            .padding(30)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_google_auth(&self) -> Element<Message> {
        let title = Text::new("🔐 Accedi con Google").size(44).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        
        let subtitle = Text::new("Autentica il tuo account Google per continuare").size(18).font(ROBOTO_FONT);

        let google_button = Button::new(
            Container::new(
                Text::new("🔗 Accedi con Google").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(340.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(1.0, 1.0, 1.0), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.98, 0.98, 0.98), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.15) }),
                    Status::Pressed => (Color::from_rgb(0.95, 0.95, 0.95), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.8, 0.8, 0.8), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::BLACK,
                    border: Border::default().rounded(28).width(2.0).color(Color::from_rgb(0.2, 0.2, 0.2)),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::StartGoogleAuth);

        let error_message = if let Some(ref error) = self.auth_error {
            Text::new(format!("⚠️ Errore: {}", error)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let back_button = Button::new(
            Container::new(
                Text::new("← Indietro").size(14).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(140.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.6, 0.6, 0.6), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.7, 0.7, 0.7), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::BackToHome);

        let content = Column::new()
            .push(Space::new().height(Length::Fixed(100.0)))
            .push(title)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(subtitle)
            .push(Space::new().height(Length::Fixed(60.0)))
            .push(google_button)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(error_message)
            .push(Space::new().height(Length::Fill))
            .push(back_button)
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_choose_nickname(&self) -> Element<Message> {
        let title = Text::new("✨ Scegli il Tuo Nickname").size(40).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        let subtitle = Text::new("Questo sarà il nome con cui gli altri ti riconosceranno").size(16).font(ROBOTO_FONT);

        // Show Google account info
        let google_info = if let Some(ref google_data) = self.google_auth_data {
            Text::new(format!("Account Google: {}", google_data.email)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let error_message = if let Some(ref error) = self.auth_error {
            Text::new(format!("⚠️ {}", error)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let input = TextInput::new("Inserisci il tuo nickname...", &self.nickname_input)
            .on_input(Message::NicknameChanged)
            .padding(12)
            .size(16)
            .width(Length::Fixed(350.0));

        let submit_button = Button::new(
            Container::new(
                Text::new("✓ Crea Identità").size(17).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(300.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.11, 0.42, 0.87), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.15, 0.50, 0.95), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.08, 0.35, 0.75), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::NicknameSubmit);

        let back_button = Button::new(
            Container::new(
                Text::new("← Indietro").size(14).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(140.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.6, 0.6, 0.6), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.7, 0.7, 0.7), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::BackToHome);

        let button_row = Row::new()
            .push(back_button)
            .push(Space::new().width(Length::Fill))
            .push(submit_button)
            .width(Length::Fill)
            .spacing(15);

        let content = Column::new()
            .push(Space::new().height(Length::Fill))
            .push(title)
            .push(subtitle)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(google_info)
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(input)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(error_message)
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(button_row)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_saving(&self) -> Element<Message> {
        let title = Text::new("Configurazione in corso...").size(36).font(ROBOTO_FONT);
        
        // SVG spinner with smooth rotation at 60fps (6° per frame = 60 frames for 360°)
        let angle = (self.spinner_frame * 6) % 360;
        let svg_content = format!(
            r##"<svg width="100" height="100" viewBox="0 0 50 50" xmlns="http://www.w3.org/2000/svg">
                <g transform="rotate({} 25 25)">
                    <path fill="#ffffff" d="M41.9 23.9c-.3-6.1-4-11.8-9.5-14.4-6-2.7-13.3-1.6-18.3 2.6-4.8 4-7 10.5-5.6 16.6 1.3 6 6 10.9 11.9 12.5 7.1 2 13.6-1.4 17.6-7.2-3.6 4.8-9.1 8-15.2 6.9-6.1-1.1-11.1-5.7-12.5-11.7-1.5-6.4 1.5-13.1 7.2-16.4 5.9-3.4 14.2-2.1 18.1 3.7 1 1.4 1.7 3.1 2 4.8.3 1.4.2 2.9.4 4.3.2 1.3 1.3 3 2.8 2.1 1.3-.8 1.2-2.5 1.1-3.8 0-.4.1.7 0 0z"/>
                </g>
            </svg>"##,
            angle
        );
        
        let spinner = Svg::new(
            svg::Handle::from_memory(svg_content.into_bytes())
        )
        .width(Length::Fixed(100.0))
        .height(Length::Fixed(100.0));
        
        let content = Column::new()
            .push(Space::new().height(Length::Fill))
            .push(title)
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(spinner)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_main_app(&self) -> Element<Message> {
        let welcome = Text::new("🦦 Benvenuto su Otter!").size(48).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        
        let user_info = if let Some(ref identity) = self.user_identity {
            Text::new(format!("Nickname: {}", identity.nickname)).size(20).font(ROBOTO_FONT)
        } else {
            Text::new("Identità: Sconosciuta").size(20).font(ROBOTO_FONT)
        };

        let email_info = if let Some(ref identity) = self.user_identity {
            Text::new(format!("Email: {}", identity.google_email)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let id_info = if let Some(ref identity) = self.user_identity {
            let short_id = if identity.id.len() > 8 {
                &identity.id[..8]
            } else {
                &identity.id
            };
            Text::new(format!("ID: {}", short_id)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let peer_info = if let Some(ref identity) = self.user_identity {
            let short_peer = if identity.peer_id.len() > 16 {
                &identity.peer_id[..16]
            } else {
                &identity.peer_id
            };
            Text::new(format!("Peer ID: {}...", short_peer)).size(14).font(ROBOTO_FONT)
        } else {
            Text::new("").size(14).font(ROBOTO_FONT)
        };

        let status = Text::new("✓ Pronto per chattare").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        
        let placeholder = Text::new(
            "Interfaccia di chat principale in arrivo...\n\n\
Funzionalità in sviluppo:\n\
• Scoperta di peer e connessione\n\
• Messaggistica crittografata end-to-end\n\
• Chiamate vocali sicure\n\
• Gestione dei contatti e dell'identità"
        ).size(16).width(Length::Fill).font(ROBOTO_FONT);

        let logout_btn = Button::new(
            Container::new(
                Text::new("🚭 Esci").size(15).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([16, 12])
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(56.0))
            .style(|_theme, status| {
                let (color, shadow) = match status {
                    Status::Active => (Color::from_rgb(0.8, 0.2, 0.2), Shadow::default()),
                    Status::Hovered => (Color::from_rgb(0.95, 0.3, 0.3), Shadow { offset: iced::Vector::new(0.0, 2.0), blur_radius: 8.0, color: Color::from_rgba(0.0, 0.0, 0.0, 0.3) }),
                    Status::Pressed => (Color::from_rgb(0.7, 0.1, 0.1), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.5, 0.5, 0.5), Shadow::default()),
                };
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(28),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::Logout);

        let content = Column::new()
            .push(Space::new().height(Length::Fixed(40.0)))
            .push(welcome)
            .push(user_info)
            .push(email_info)
            .push(id_info)
            .push(peer_info)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(status)
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(placeholder)
            .push(Space::new().height(Length::Fill))
            .push(logout_btn)
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
