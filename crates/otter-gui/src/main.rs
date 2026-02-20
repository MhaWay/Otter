#![windows_subsystem = "windows"]

use iced::{
    widget::{Column, Container, Row, Scrollable, Text, Button, Space, TextInput, text, button, container, Svg},
    Element, Length, Font, Alignment, Task, Border,
    Background, Color, Shadow,
    time, Subscription,
    event, mouse, Event,
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
use tokio::sync::mpsc;
use otter_network::{NetworkEvent, NetworkCommand, Network, create_network_channels};
use futures::stream;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use once_cell::sync::Lazy;

// Canale globale per eventi di rete
// Il task di rete scrive qui, la subscription legge da qui
static NETWORK_EVENTS: Lazy<(mpsc::UnboundedSender<NetworkEvent>, Arc<TokioMutex<mpsc::UnboundedReceiver<NetworkEvent>>>)> = 
    Lazy::new(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Arc::new(TokioMutex::new(rx)))
    });

// OAuth credentials loaded from environment variables
// Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables
const REDIRECT_URI: &str = "http://localhost:8080";

const ROBOTO_FONT: Font = Font::with_name("Roboto");

// Source - https://stackoverflow.com/a/79782372
// Posted by Péter Szilvási, modified by community. See post 'Timeline' for change history
// Retrieved 2026-02-17, License - CC BY-SA 4.0

fn main() -> iced::Result {
    // Carica le variabili d'ambiente dal file .env (se esiste)
    // Ignora l'errore se il file non esiste
    let _ = dotenvy::dotenv();
    
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
    RegistrationIdentityCheck(Result<(), String>), // Ok se non esiste (può registrarsi), Err se esiste già
    NicknameChanged(String),
    NicknameSubmit,
    IdentitySaved(Result<Identity, String>),
    ContactsSaved(Result<(), String>),
    ContactsLoaded(Result<Vec<Contact>, String>),
    SpinnerTick,
    BackToHome,
    Logout,
    ChangeTab(MainAppTab),
    ToggleSidebar,
    StartSidebarDrag,
    DragSidebar(f32),
    EndSidebarDrag,
    
    // Gestione contatti
    ChangeContactsSubTab(ContactsSubTab),
    PeerSearchQueryChanged(String),
    SendContactRequest(String, String), // peer_id, nickname
    AcceptContactRequest(String),       // peer_id
    RejectContactRequest(String),       // peer_id
    BlockContact(String),                // peer_id
    UnblockContact(String),              // peer_id
    
    // Network events
    NetworkStarted(Result<mpsc::Sender<NetworkCommand>, String>),
    NetworkEvent(NetworkEvent),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MainAppTab {
    Home,
    Contacts,
    Profile,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ContactsSubTab {
    ContactsList,      // Lista contatti confermati
    AddContact,        // Cerca e aggiungi nuovi contatti
    PendingRequests,   // Richieste in arrivo/in uscita
    BlockedContacts,   // Contatti bloccati
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize, Default)]
enum ContactStatus {
    #[default]
    Pending,     // Richiesta inviata, in attesa
    Accepted,    // Contatto confermato
    Rejected,    // Richiesta rifiutata
    Blocked,     // Contatto bloccato
    Incoming,    // Richiesta ricevuta, in attesa di risposta
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Contact {
    peer_id: String,
    nickname: String,
    avatar: Option<String>,  // base64 o URL
    #[serde(skip)]
    status: ContactStatus,
    last_seen: Option<String>,
    is_online: bool,
    #[serde(default)]
    discovered_at: Option<String>,  // Timestamp scoperta
}

#[derive(Debug, Clone)]
struct ContactRequest {
    peer_id: String,
    nickname: String,
    message: Option<String>,
    timestamp: String,
    incoming: bool,  // true = richiesta ricevuta, false = richiesta inviata
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
    current_tab: MainAppTab,
    sidebar_expanded: bool,
    sidebar_width: f32,
    is_dragging_sidebar: bool,
    
    // Gestione contatti
    contacts_sub_tab: ContactsSubTab,
    contacts_list: Vec<Contact>,
    contact_requests: Vec<ContactRequest>,
    blocked_contacts: Vec<Contact>,
    
    // Ricerca peer
    peer_search_query: String,
    peer_search_results: Vec<Contact>,
    discovered_peers: Vec<Contact>,  // Peer CONNESSI ora (RAM, non persistente)
    peers_history: Vec<Contact>,     // Cronologia peer da file (caricata on-demand)
    
    // Network P2P
    network_command_tx: Option<mpsc::Sender<NetworkCommand>>,
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
            current_tab: MainAppTab::Home,
            sidebar_expanded: false,
            sidebar_width: 240.0,
            is_dragging_sidebar: false,
            
            // Contatti
            contacts_sub_tab: ContactsSubTab::ContactsList,
            contacts_list: Vec::new(),
            contact_requests: Vec::new(),
            blocked_contacts: Vec::new(),
            
            // Ricerca
            peer_search_query: String::new(),
            peer_search_results: Vec::new(),
            // Peer scoperti dalla rete P2P
            discovered_peers: Vec::new(),
            peers_history: Vec::new(),
            
            // Network P2P
            network_command_tx: None,
        }
    }
}

impl GuiApp {
    fn init_discovered_peers_example() -> Vec<Contact> {
        // Temporary example peers for UI testing when network is not running
        vec![
                Contact {
                    peer_id: "12D3KooWA8EXV3KjBxEU5EnsPfneLx84vMWAtTBQBeyooN8uEzg1".to_string(),
                    nickname: "Alice".to_string(),
                    avatar: None,
                    status: ContactStatus::Accepted,
                    last_seen: Some("Online ora".to_string()),
                    is_online: true,
                    discovered_at: None,
                },
                Contact {
                    peer_id: "12D3KooWBvvVGDJXhQpGwmCCV1gyDZN8vpPBjXjEeFqB8xbS9z7H".to_string(),
                    nickname: "Bob".to_string(),
                    avatar: None,
                    status: ContactStatus::Accepted,
                    last_seen: Some("5 min fa".to_string()),
                    is_online: true,
                    discovered_at: None,
                },
                Contact {
                    peer_id: "12D3KooWGKYjWB5JKCJCrfP9xqUa4DmPmFXZP3PG5vQY8sHXx5bU".to_string(),
                    nickname: "Carol".to_string(),
                    avatar: None,
                    status: ContactStatus::Accepted,
                    last_seen: Some("Online ora".to_string()),
                    is_online: true,
                    discovered_at: None,
                },
                Contact {
                    peer_id: "12D3KooWRZKYXkpqXtN7c7QdVmQ8NqwJVFmvzPXvCKzKwHCPzN2M".to_string(),
                    nickname: "David".to_string(),
                    avatar: None,
                    status: ContactStatus::Accepted,
                    last_seen: Some("1 ora fa".to_string()),
                    is_online: false,
                    discovered_at: None,
                },
        ]
    }

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
            // Avvia la rete P2P
            return (app, Self::start_network_task());
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

    // Controlla se esiste già un'identità su Google Drive (per evitare doppie registrazioni)
    async fn check_identity_exists_on_drive(access_token: String) -> Result<(), String> {
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
                // Cartella .otter non esiste, quindi nessuna identità esistente
                return Ok(());
            }
        } else {
            return Ok(());
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
        
        if let Some(files) = file_result["files"].as_array() {
            if files.first().is_some() {
                // Identità trovata! Questo è un errore durante la registrazione
                return Err("Questo profilo Google ha già un'identità Otter esistente.\n\n\
                           Per accedere, clicca su \"Accedi\".\n\n\
                           Se vuoi creare una nuova identità, devi prima eliminare quella esistente:\n\
                           • Vai su Google Drive\n\
                           • Trova e elimina la cartella .otter".to_string());
            }
        }
        
        // Nessuna identità trovata, può procedere con la registrazione
        Ok(())
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
    
    async fn save_contacts_to_drive(
        contacts: Vec<Contact>,
        access_token: String,
    ) -> Result<(), String> {
        let client = reqwest::Client::new();
        
        // Cerca cartella .otter
        let search_response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", "name='.otter' and mimeType='application/vnd.google-apps.folder' and trashed=false")])
            .header("Authorization", format!("Bearer {}", &access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca cartella: {}", e))?;
        
        let search_result: serde_json::Value = search_response.json().await
            .map_err(|e| format!("Errore parsing risposta: {}", e))?;
        
        // Preleva o crea folder ID
        let folder_id = if let Some(files) = search_result["files"].as_array() {
            if let Some(folder) = files.first() {
                folder["id"].as_str().unwrap_or("").to_string()
            } else {
                return Err("Cartella .otter non trovata".to_string());
            }
        } else {
            return Err("Formato risposta Drive invalido".to_string());
        };
        
        // Cerca file contacts.json esistente
        let search_file = format!(
            "name='contacts.json' and parents='{}' and trashed=false",
            folder_id
        );
        let file_response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", &search_file)])
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca file: {}", e))?;
        
        let file_result: serde_json::Value = file_response.json().await
            .map_err(|e| format!("Errore parsing file: {}", e))?;
        
        let file_id = if let Some(files) = file_result["files"].as_array() {
            files.first().and_then(|f| f["id"].as_str()).map(|s| s.to_string())
        } else {
            None
        };
        
        let contacts_json = serde_json::to_string_pretty(&contacts)
            .map_err(|e| format!("Errore serializzazione contatti: {}", e))?;
        
        // Upload o update
        if let Some(fid) = file_id {
            // Update file
            let response = client
                .patch(format!("https://www.googleapis.com/upload/drive/v3/files/{}", fid))
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .body(contacts_json)
                .send()
                .await
                .map_err(|e| format!("Errore update: {}", e))?;
            
            if !response.status().is_success() {
                return Err("Errore update contatti su Drive".to_string());
            }
        } else {
            // Create new file
            let metadata = serde_json::json!({
                "name": "contacts.json",
                "mimeType": "application/json",
                "parents": [folder_id]
            });
            
            let boundary = "foo_bar_baz";
            let body = format!(
                "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n--{boundary}\r\nContent-Type: application/json\r\n\r\n{}\r\n--{boundary}--",
                serde_json::to_string(&metadata).unwrap(),
                contacts_json
            );
            
            let response = client
                .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", format!("multipart/related; boundary={}", boundary))
                .body(body)
                .send()
                .await
                .map_err(|e| format!("Errore upload: {}", e))?;
            
            if !response.status().is_success() {
                return Err("Errore creazione contatti su Drive".to_string());
            }
        }
        
        tracing::info!("Contatti salvati su Google Drive");
        Ok(())
    }
    
    async fn load_contacts_from_drive(access_token: String) -> Result<Vec<Contact>, String> {
        let client = reqwest::Client::new();
        
        // Cerca cartella .otter
        let search_response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", "name='.otter' and mimeType='application/vnd.google-apps.folder' and trashed=false")])
            .header("Authorization", format!("Bearer {}", &access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca cartella: {}", e))?;
        
        let search_result: serde_json::Value = search_response.json().await
            .map_err(|e| format!("Errore parsing risposta: {}", e))?;
        
        let folder_id = if let Some(files) = search_result["files"].as_array() {
            if let Some(folder) = files.first() {
                folder["id"].as_str().unwrap_or("")
            } else {
                return Ok(Vec::new()); // Cartella non esiste ancora
            }
        } else {
            return Ok(Vec::new());
        };
        
        // Cerca file contacts.json
        let search_file = format!(
            "name='contacts.json' and parents='{}' and trashed=false",
            folder_id
        );
        let file_response = client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[("q", &search_file)])
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| format!("Errore ricerca file: {}", e))?;
        
        let file_result: serde_json::Value = file_response.json().await
            .map_err(|e| format!("Errore parsing file: {}", e))?;
        
        if let Some(files) = file_result["files"].as_array() {
            if let Some(file) = files.first() {
                if let Some(file_id) = file["id"].as_str() {
                    // Scarica il file
                    let download_response = client
                        .get(format!("https://www.googleapis.com/drive/v3/files/{}?alt=media", file_id))
                        .header("Authorization", format!("Bearer {}", access_token))
                        .send()
                        .await
                        .map_err(|e| format!("Errore download: {}", e))?;
                    
                    let content = download_response.text().await
                        .map_err(|e| format!("Errore lettura: {}", e))?;
                    
                    let contacts: Vec<Contact> = serde_json::from_str(&content)
                        .map_err(|e| format!("Errore parsing contatti: {}", e))?;
                    
                    tracing::info!("Caricati {} contatti da Google Drive", contacts.len());
                    return Ok(contacts);
                }
            }
        }
        
        Ok(Vec::new())
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
        // Load OAuth credentials from environment variables (.env file)
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .unwrap_or_else(|_| {
                // Public Client ID - safe to distribute in app
                "251946123352-bp2baikvt4817semo2d541dd2ffov6lk.apps.googleusercontent.com".to_string()
            });
        
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| "Google OAuth non configurato. Copia .env.example in .env e aggiungi GOOGLE_CLIENT_SECRET.".to_string())?;
        
        tracing::info!("Usando Client ID: {}", client_id);
        
        // Valida che il Client ID sia stato configurato
        if client_id.contains("YOUR_CLIENT_ID") {
            return Err("Google OAuth non configurato. Vedi OAUTH_SETUP.md per istruzioni.".to_string());
        }
        
        // Create OAuth2 client (PKCE + ClientSecret for installed apps)
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),  // Required for installed apps
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
        tracing::debug!("PKCE challenge generato con successo");

        // Generate authorization URL
        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/drive.file".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        
        tracing::info!("Auth URL generato: {}", auth_url.as_str());

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
        tracing::info!("Effettuo scambio token con Google...");
        tracing::debug!("Authorization code: {}", code.secret());
        
        let token_result = client
            .exchange_code(code)
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                let error_msg = format!("Errore scambio token: {:?}", e);
                tracing::error!("{}", error_msg);
                error_msg
            })?;
        
        tracing::info!("Token ricevuto con successo");

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
                // After successful Google auth, store auth data and try to load identity from Drive
                self.google_auth_data = Some(google_data.clone());
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
                        
                        // Load contacts from Google Drive after successful identity load
                        if let Some(auth_data) = &self.google_auth_data {
                            let token = auth_data.access_token.clone();
                            return Task::perform(
                                Self::load_contacts_from_drive(token),
                                Message::ContactsLoaded
                            );
                        }
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
                // Prima di procedere con la registrazione, controlla se esiste già un'identità
                self.google_auth_data = Some(data.clone());
                self.auth_error = None;
                
                let access_token = data.access_token.clone();
                return Task::perform(
                    async move {
                        GuiApp::check_identity_exists_on_drive(access_token).await
                    },
                    Message::RegistrationIdentityCheck
                );
            }
            Message::RegistrationIdentityCheck(result) => {
                match result {
                    Ok(()) => {
                        // Nessuna identità esistente, può procedere con la registrazione
                        self.current_screen = Screen::ChooseNickname;
                    }
                    Err(error) => {
                        // Identità già esistente! Torna alla home con errore
                        self.current_screen = Screen::Home;
                        self.auth_error = Some(error);
                        self.google_auth_data = None;
                    }
                }
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
                        // Avvia la rete P2P
                        return Self::start_network_task();
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
            Message::ChangeTab(tab) => {
                self.current_tab = tab;
            }
            Message::ToggleSidebar => {
                self.sidebar_expanded = !self.sidebar_expanded;
            }
            Message::StartSidebarDrag => {
                if self.sidebar_expanded {
                    self.is_dragging_sidebar = true;
                }
            }
            Message::DragSidebar(x) => {
                if self.is_dragging_sidebar {
                    // Limita la larghezza tra 200 e 500 pixel
                    self.sidebar_width = x.max(200.0).min(500.0);
                }
            }
            Message::EndSidebarDrag => {
                self.is_dragging_sidebar = false;
            }
            
            // Gestione contatti
            Message::ChangeContactsSubTab(sub_tab) => {
                self.contacts_sub_tab = sub_tab;
            }
            Message::PeerSearchQueryChanged(query) => {
                self.peer_search_query = query.clone();
                
                // Simula ricerca peer (in futuro: query alla rete P2P)
                self.peer_search_results = self.simulate_peer_search(&query);
            }
            Message::SendContactRequest(peer_id, nickname) => {
                // TODO: Inviare richiesta tramite protocollo P2P
                let request = ContactRequest {
                    peer_id: peer_id.clone(),
                    nickname: nickname.clone(),
                    message: Some("Vorrei aggiungerti ai contatti".to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    incoming: false,
                };
                self.contact_requests.push(request);
                
                // Aggiungi anche alla lista contatti con stato Pending
                let contact = Contact {
                    peer_id,
                    nickname,
                    avatar: None,
                    status: ContactStatus::Pending,
                    last_seen: None,
                    is_online: false,
                    discovered_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                self.contacts_list.push(contact);
                
                // Salva contatti su Google Drive se autenticato
                if let Some(auth_data) = &self.google_auth_data {
                    let contacts = self.contacts_list.clone();
                    let token = auth_data.access_token.clone();
                    return Task::perform(
                        Self::save_contacts_to_drive(contacts, token),
                        Message::ContactsSaved
                    );
                }
            }
            Message::AcceptContactRequest(peer_id) => {
                // Trova e rimuovi la richiesta
                self.contact_requests.retain(|r| r.peer_id != peer_id);
                
                // Aggiorna o aggiungi il contatto
                if let Some(contact) = self.contacts_list.iter_mut().find(|c| c.peer_id == peer_id) {
                    contact.status = ContactStatus::Accepted;
                } else {
                    // Crea nuovo contatto se non esiste
                    if let Some(req) = self.contact_requests.iter().find(|r| r.peer_id == peer_id) {
                        let contact = Contact {
                            peer_id: req.peer_id.clone(),
                            nickname: req.nickname.clone(),
                            avatar: None,
                            status: ContactStatus::Accepted,
                            last_seen: None,
                            is_online: false,
                            discovered_at: Some(chrono::Utc::now().to_rfc3339()),
                        };
                        self.contacts_list.push(contact);
                    }
                }
            }
            Message::RejectContactRequest(peer_id) => {
                self.contact_requests.retain(|r| r.peer_id != peer_id);
            }
            Message::BlockContact(peer_id) => {
                // Rimuovi dalla lista contatti
                if let Some(idx) = self.contacts_list.iter().position(|c| c.peer_id == peer_id) {
                    let mut contact = self.contacts_list.remove(idx);
                    contact.status = ContactStatus::Blocked;
                    self.blocked_contacts.push(contact);
                }
            }
            Message::UnblockContact(peer_id) => {
                if let Some(idx) = self.blocked_contacts.iter().position(|c| c.peer_id == peer_id) {
                    let mut contact = self.blocked_contacts.remove(idx);
                    contact.status = ContactStatus::Accepted;
                    self.contacts_list.push(contact);
                }
            }
            
            // Gestione salvataggio contatti
            Message::ContactsSaved(result) => {
                match result {
                    Ok(_) => {
                        tracing::info!("Contatti salvati su Google Drive con successo");
                    }
                    Err(e) => {
                        tracing::error!("Errore salvataggio contatti: {}", e);
                        self.auth_error = Some(format!("Errore sync contatti: {}", e));
                    }
                }
            }
            Message::ContactsLoaded(result) => {
                match result {
                    Ok(contacts) => {
                        self.contacts_list = contacts;
                        tracing::info!("Caricati {} contatti da Google Drive", self.contacts_list.len());
                    }
                    Err(e) => {
                        tracing::warn!("Errore caricamento contatti da Drive: {}", e);
                        // Non è un errore fatale, continua con contatti vuoti
                    }
                }
            }
            
            // Network events
            Message::NetworkStarted(result) => {
                match result {
                    Ok(cmd_tx) => {
                        self.network_command_tx = Some(cmd_tx);
                        tracing::info!("Rete P2P avviata con successo");
                    }
                    Err(e) => {
                        tracing::error!("Errore avvio rete P2P: {}", e);
                        self.auth_error = Some(format!("Errore rete: {}", e));
                    }
                }
            }
            Message::NetworkEvent(event) => {
                match event {
                    NetworkEvent::PeerDiscovered { peer_id, addresses } => {
                        tracing::info!("Peer scoperto: {} con indirizzi: {:?}", peer_id, addresses);
                        
                        let peer_id_str = peer_id.to_string();
                        
                        // Aggiungi ai discovered_peers se non presente
                        if !self.discovered_peers.iter().any(|p| p.peer_id == peer_id_str) {
                            // TODO: In futuro, richiedere nickname tramite protocollo Handshake
                            // Per ora usa l'ID troncato come placeholder nickname
                            let nickname = format!("Peer_{}", &peer_id_str[..8]);
                            
                            let contact = Contact {
                                peer_id: peer_id_str,
                                nickname,
                                avatar: None,
                                status: ContactStatus::Accepted,
                                last_seen: Some("Scoperto ora".to_string()),
                                is_online: true,
                                discovered_at: Some(chrono::Utc::now().to_rfc3339()),
                            };
                            self.discovered_peers.push(contact);
                            
                            // Aggiorna risultati di ricerca se c'è una query attiva
                            if !self.peer_search_query.is_empty() {
                                self.peer_search_results = self.simulate_peer_search(&self.peer_search_query);
                            }
                        }
                    }
                    NetworkEvent::PeerConnected { peer_id } => {
                        tracing::info!("Peer connesso: {}", peer_id);
                        // Aggiorna stato online del peer
                        if let Some(peer) = self.discovered_peers.iter_mut().find(|p| p.peer_id == peer_id.to_string()) {
                            peer.is_online = true;
                            peer.last_seen = Some("Online ora".to_string());
                        }
                    }
                    NetworkEvent::PeerDisconnected { peer_id } => {
                        tracing::info!("Peer disconnesso: {}", peer_id);
                        // Aggiorna stato offline del peer
                        if let Some(peer) = self.discovered_peers.iter_mut().find(|p| p.peer_id == peer_id.to_string()) {
                            peer.is_online = false;
                            peer.last_seen = Some("Offline".to_string());
                        }
                    }
                    NetworkEvent::ListeningOn { address } => {
                        tracing::info!("Network in ascolto su: {}", address);
                    }
                    _ => {
                        // Altri eventi di rete
                        tracing::debug!("Network event: {:?}", event);
                    }
                }
            }
        }
        Task::none()
    }
    
    // Avvia la rete P2P in un task separato
     // Avvia la rete P2P in un task separato
    fn start_network_task() -> Task<Message> {
        Task::perform(
            async move {
                let (event_tx, mut event_rx, command_tx, command_rx) = create_network_channels();
                
                // Spawn task per inoltrare eventi al canale globale
                let global_events_tx = NETWORK_EVENTS.0.clone();
                tokio::spawn(async move {
                    while let Some(event) = event_rx.recv().await {
                        if global_events_tx.send(event).is_err() {
                            tracing::error!("Canale globale eventi chiuso");
                            break;
                        }
                    }
                });
                
                // Crea e avvia la rete
                let network = match Network::new(event_tx, command_rx) {
                    Ok(mut net) => {
                        // Inizia ad ascoltare su un indirizzo locale
                        if let Err(e) = net.listen("/ip4/0.0.0.0/tcp/0") {
                            return Err(format!("Errore listen: {}", e));
                        }
                        net
                    }
                    Err(e) => {
                        return Err(format!("Errore creazione rete: {}", e));
                    }
                };
                
                // Avvia il network loop in un task separato
                tokio::spawn(async move {
                    if let Err(e) = network.run().await {
                        tracing::error!("Errore network loop: {}", e);
                    }
                });
                
                Ok(command_tx)
            },
            |result| Message::NetworkStarted(result)
        )
    }
    
    // Cerca peer scoperti nella rete P2P
    // Attualmente cerca tra i discovered_peers (simulati)
    // TODO: In futuro, discovered_peers sarà popolato da eventi NetworkEvent::PeerDiscovered
    fn simulate_peer_search(&self, query: &str) -> Vec<Contact> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        
        let query_lower = query.to_lowercase();
        
        // Cerca tra i peer scoperti dalla rete P2P
        self.discovered_peers
            .iter()
            .filter(|p| {
                // Filtra peer già presenti nei contatti
                let already_contact = self.contacts_list.iter().any(|c| c.peer_id == p.peer_id);
                let already_requested = self.contact_requests.iter().any(|r| r.peer_id == p.peer_id);
                let is_blocked = self.blocked_contacts.iter().any(|b| b.peer_id == p.peer_id);
                
                // Mostra solo se non è già contatto/richiesto/bloccato e corrisponde alla query
                !already_contact && !already_requested && !is_blocked &&
                (p.nickname.to_lowercase().contains(&query_lower) ||
                 p.peer_id.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    fn subscription(&self) -> Subscription<Message> {
        let spinner_sub = if self.current_screen == Screen::Saving {
            time::every(Duration::from_millis(16))
                .map(|_| Message::SpinnerTick)
        } else {
            Subscription::none()
        };
        
        let mouse_sub = if self.is_dragging_sidebar {
            event::listen_with(|event, _status, _id| {
                match event {
                    Event::Mouse(mouse::Event::CursorMoved { position }) => {
                        Some(Message::DragSidebar(position.x))
                    }
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        Some(Message::EndSidebarDrag)
                    }
                    _ => None,
                }
            })
        } else {
            Subscription::none()
        };
        
        // Network events subscription  
        let network_sub = if self.current_screen == Screen::MainApp {
            Subscription::run(|| {
                stream::unfold((), |_| async {
                    // Leggi dal canale globale
                    let mut rx = NETWORK_EVENTS.1.lock().await;
                    match rx.recv().await {
                        Some(event) => Some((Message::NetworkEvent(event), ())),
                        None => None,
                    }
                })
            })
        } else {
            Subscription::none()
        };
        
        Subscription::batch([spinner_sub, mouse_sub, network_sub])
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
        // Sidebar con icone rotonde
        let sidebar = self.view_sidebar();
        
        // Contenuto principale basato sul tab selezionato
        let main_content = match self.current_tab {
            MainAppTab::Home => self.view_home_tab(),
            MainAppTab::Contacts => self.view_contacts_tab(),
            MainAppTab::Profile => self.view_profile_tab(),
            MainAppTab::Settings => self.view_settings_tab(),
        };
        
        // Handle per ridimensionare la sidebar (visibile solo quando espansa)
        let resize_handle: Element<Message> = if self.sidebar_expanded {
            let handle_btn: Element<Message> = Button::new(Text::new(""))
                .width(Length::Fixed(4.0))
                .height(Length::Fill)
                .style(|_theme, status| {
                    let color = match status {
                        Status::Hovered | Status::Pressed => Color::from_rgb(0.4, 0.6, 1.0),
                        _ => Color::from_rgb(0.25, 0.25, 0.27),
                    };
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::WHITE,
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: false,
                    }
                })
                .on_press(Message::StartSidebarDrag)
                .into();
            handle_btn
        } else {
            Space::new().width(Length::Fixed(0.0)).into()
        };
        
        // Layout principale: sidebar + handle + contenuto
        let layout = Row::new()
            .push(sidebar)
            .push(resize_handle)
            .push(main_content)
            .width(Length::Fill)
            .height(Length::Fill);
        
        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_sidebar(&self) -> Element<Message> {
        let sidebar_width = if self.sidebar_expanded { self.sidebar_width } else { 80.0 };
        
        // Pulsante Home (Base Page)
        let home_btn = self.create_sidebar_button("🏠", MainAppTab::Home, "Home");
        
        // Separatore dinamico
        let separator = Container::new(Space::new())
            .width(Length::Fixed(sidebar_width - 20.0))
            .height(Length::Fixed(2.0))
            .style(|_theme| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.3))),
                    ..Default::default()
                }
            });
        
        // Lista contatti
        let contacts_btn = self.create_sidebar_button("👥", MainAppTab::Contacts, "Contatti");
        
        // Spazio flessibile per spingere i bottoni in basso
        let spacer = Space::new().height(Length::Fill);
        
        // Separatore prima dei pulsanti in fondo
        let bottom_separator = Container::new(Space::new())
            .width(Length::Fixed(sidebar_width - 20.0))
            .height(Length::Fixed(2.0))
            .style(|_theme| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.3))),
                    ..Default::default()
                }
            });
        
        // Pulsante Profilo
        let profile_btn = self.create_sidebar_button("👤", MainAppTab::Profile, "Profilo");
        
        // Pulsante Impostazioni
        let settings_btn = self.create_sidebar_button("⚙️", MainAppTab::Settings, "Impostazioni");
        
        // Pulsante toggle espansione sidebar
        let toggle_icon = if self.sidebar_expanded { "◀" } else { "▶" };
        let toggle_btn = Button::new(
            Container::new(
                Text::new(toggle_icon)
                    .size(14)
                    .font(ROBOTO_FONT)
                    .shaping(text::Shaping::Advanced)
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .width(Length::Fixed(sidebar_width))
        .height(Length::Fixed(20.0))
        .style(|_theme, status| {
            let (color, shadow) = match status {
                Status::Active => (Color::from_rgb(0.15, 0.15, 0.17), Shadow::default()),
                Status::Hovered => (Color::from_rgb(0.20, 0.20, 0.22), Shadow {
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 4.0,
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.2)
                }),
                Status::Pressed => (Color::from_rgb(0.10, 0.10, 0.12), Shadow::default()),
                Status::Disabled => (Color::from_rgb(0.3, 0.3, 0.3), Shadow::default()),
            };
            button::Style {
                background: Some(Background::Color(color)),
                text_color: Color::from_rgb(0.7, 0.7, 0.7),
                border: Border::default(),
                shadow,
                snap: false,
            }
        })
        .on_press(Message::ToggleSidebar);
        
        let sidebar_content = Column::new()
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(home_btn)
            .push(Space::new().height(Length::Fixed(5.0)))
            .push(separator)
            .push(Space::new().height(Length::Fixed(5.0)))
            .push(contacts_btn)
            .push(spacer)
            .push(bottom_separator)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(profile_btn)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(settings_btn)
            .push(Space::new().height(Length::Fixed(10.0)))
            .push(toggle_btn)
            .width(Length::Fixed(sidebar_width))
            .height(Length::Fill)
            .align_x(if self.sidebar_expanded { Alignment::Start } else { Alignment::Center });
        
        Container::new(sidebar_content)
            .width(Length::Fixed(sidebar_width))
            .height(Length::Fill)
            .style(|_theme| {
                container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
                    ..Default::default()
                }
            })
            .into()
    }
    
    fn create_sidebar_button(&self, icon: &str, tab: MainAppTab, label: &str) -> Element<Message> {
        let is_active = self.current_tab == tab;
        let icon_str = icon.to_string();
        let label_str = label.to_string();
        
        if self.sidebar_expanded {
            let button_width = self.sidebar_width - 10.0; // Margine di 10px
            
            // Versione espansa: icona + testo allineato a sinistra
            let content = Row::new()
                .push(
                    Text::new(icon_str)
                        .size(28)
                        .font(ROBOTO_FONT)
                        .shaping(text::Shaping::Advanced)
                )
                .push(Space::new().width(Length::Fixed(12.0)))
                .push(
                    Text::new(label_str)
                        .size(16)
                        .font(ROBOTO_FONT)
                )
                .align_y(Alignment::Center)
                .padding([0, 15]);
            
            let button = Button::new(
                Container::new(content)
                    .align_x(iced::alignment::Horizontal::Left)
                    .align_y(iced::alignment::Vertical::Center)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .width(Length::Fixed(button_width))
            .height(Length::Fixed(50.0))
            .style(move |_theme, status| {
                let base_color = if is_active {
                    Color::from_rgb(0.20, 0.45, 0.90)
                } else {
                    Color::from_rgb(0.18, 0.18, 0.20)
                };
                
                let (color, shadow) = match status {
                    Status::Active => (base_color, Shadow::default()),
                    Status::Hovered => {
                        let hover_color = if is_active {
                            Color::from_rgb(0.25, 0.50, 0.95)
                        } else {
                            Color::from_rgb(0.25, 0.25, 0.27)
                        };
                        (hover_color, Shadow {
                            offset: iced::Vector::new(0.0, 2.0),
                            blur_radius: 8.0,
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3)
                        })
                    }
                    Status::Pressed => (Color::from_rgb(0.15, 0.40, 0.85), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.3, 0.3, 0.3), Shadow::default()),
                };
                
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(25),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::ChangeTab(tab));
            
            button.into()
        } else {
            // Versione compatta: solo icona centrata
            let button = Button::new(
                Container::new(
                    Text::new(icon_str)
                        .size(28)
                        .font(ROBOTO_FONT)
                        .shaping(text::Shaping::Advanced)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            )
            .width(Length::Fixed(60.0))
            .height(Length::Fixed(60.0))
            .style(move |_theme, status| {
                let base_color = if is_active {
                    Color::from_rgb(0.20, 0.45, 0.90)
                } else {
                    Color::from_rgb(0.18, 0.18, 0.20)
                };
                
                let (color, shadow) = match status {
                    Status::Active => (base_color, Shadow::default()),
                    Status::Hovered => {
                        let hover_color = if is_active {
                            Color::from_rgb(0.25, 0.50, 0.95)
                        } else {
                            Color::from_rgb(0.25, 0.25, 0.27)
                        };
                        (hover_color, Shadow {
                            offset: iced::Vector::new(0.0, 2.0),
                            blur_radius: 8.0,
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3)
                        })
                    }
                    Status::Pressed => (Color::from_rgb(0.15, 0.40, 0.85), Shadow::default()),
                    Status::Disabled => (Color::from_rgb(0.3, 0.3, 0.3), Shadow::default()),
                };
                
                button::Style {
                    background: Some(Background::Color(color)),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(30),
                    shadow,
                    snap: false,
                }
            })
            .on_press(Message::ChangeTab(tab));
            
            button.into()
        }
    }
    
    fn view_home_tab(&self) -> Element<Message> {
        let title = Text::new("🏠 Home - Novità e Informazioni")
            .size(32)
            .font(ROBOTO_FONT)
            .shaping(text::Shaping::Advanced);
        
        let content = Text::new(
            "Benvenuto su Otter! 🦦\n\n\
            Qui troverai:\n\
            • Novità e aggiornamenti\n\
            • Informazioni sulla piattaforma\n\
            • Statistiche e dettagli\n\n\
            Funzionalità in sviluppo..."
        )
        .size(16)
        .font(ROBOTO_FONT);
        
        let layout = Column::new()
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(title)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(content)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);
        
        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_contacts_tab(&self) -> Element<Message> {
        let title = Text::new("👥 Contatti")
            .size(32)
            .font(ROBOTO_FONT)
            .shaping(text::Shaping::Advanced);
        
        // Tab navigation bar
        let tab_buttons = Row::new()
            .push(self.create_contacts_tab_button("📋 Lista Contatti", ContactsSubTab::ContactsList))
            .push(Space::new().width(Length::Fixed(10.0)))
            .push(self.create_contacts_tab_button("➕ Aggiungi Contatto", ContactsSubTab::AddContact))
            .push(Space::new().width(Length::Fixed(10.0)))
            .push(self.create_contacts_tab_button("📬 Richieste", ContactsSubTab::PendingRequests))
            .push(Space::new().width(Length::Fixed(10.0)))
            .push(self.create_contacts_tab_button("🚫 Bloccati", ContactsSubTab::BlockedContacts))
            .spacing(5);
        
        // Content based on selected sub-tab
        let sub_content = match self.contacts_sub_tab {
            ContactsSubTab::ContactsList => self.view_contacts_list(),
            ContactsSubTab::AddContact => self.view_add_contact(),
            ContactsSubTab::PendingRequests => self.view_pending_requests(),
            ContactsSubTab::BlockedContacts => self.view_blocked_contacts(),
        };
        
        let layout = Column::new()
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(title)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(tab_buttons)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(sub_content)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);
        
        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn create_contacts_tab_button(&self, label: &str, sub_tab: ContactsSubTab) -> Element<Message> {
        let is_active = self.contacts_sub_tab == sub_tab;
        let label_str = label.to_string();
        
        Button::new(
            Text::new(label_str)
                .size(14)
                .font(ROBOTO_FONT)
        )
        .padding([8, 16])
        .style(move |_theme, status| {
            let base_color = if is_active {
                Color::from_rgb(0.20, 0.45, 0.90)
            } else {
                Color::from_rgb(0.18, 0.18, 0.20)
            };
            
            let color = match status {
                Status::Active => base_color,
                Status::Hovered => {
                    if is_active {
                        Color::from_rgb(0.25, 0.50, 0.95)
                    } else {
                        Color::from_rgb(0.25, 0.25, 0.27)
                    }
                }
                Status::Pressed => Color::from_rgb(0.15, 0.40, 0.85),
                Status::Disabled => Color::from_rgb(0.3, 0.3, 0.3),
            };
            
            button::Style {
                background: Some(Background::Color(color)),
                text_color: Color::WHITE,
                border: Border::default().rounded(5),
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::ChangeContactsSubTab(sub_tab))
        .into()
    }
    
    fn view_contacts_list(&self) -> Element<Message> {
        let mut content = Column::new().spacing(10);
        
        if self.contacts_list.is_empty() {
            content = content.push(
                Text::new("Nessun contatto. Usa 'Aggiungi Contatto' per iniziare!")
                    .size(16)
                    .font(ROBOTO_FONT)
                    .color(Color::from_rgb(0.6, 0.6, 0.6))
            );
        } else {
            for contact in &self.contacts_list {
                if contact.status == ContactStatus::Accepted {
                    content = content.push(self.create_contact_item(contact, false));
                }
            }
        }
        
        Scrollable::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_add_contact(&self) -> Element<Message> {
        let search_input = TextInput::new(
            "Cerca per ID o Nickname...",
            &self.peer_search_query
        )
        .on_input(Message::PeerSearchQueryChanged)
        .padding(12)
        .size(16)
        .font(ROBOTO_FONT);
        
        let mut results = Column::new().spacing(10);
        
        if self.peer_search_query.is_empty() {
            results = results.push(
                Text::new("Inserisci un ID o nickname per cercare peer nella rete")
                    .size(14)
                    .font(ROBOTO_FONT)
                    .color(Color::from_rgb(0.6, 0.6, 0.6))
            );
        } else if self.peer_search_results.is_empty() {
            results = results.push(
                Text::new("Nessun peer trovato")
                    .size(14)
                    .font(ROBOTO_FONT)
                    .color(Color::from_rgb(0.6, 0.6, 0.6))
            );
        } else {
            for peer in &self.peer_search_results {
                results = results.push(self.create_peer_search_result(peer));
            }
        }
        
        Column::new()
            .push(search_input)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(Scrollable::new(results).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_pending_requests(&self) -> Element<Message> {
        let mut content = Column::new().spacing(10);
        
        if self.contact_requests.is_empty() {
            content = content.push(
                Text::new("Nessuna richiesta pending")
                    .size(16)
                    .font(ROBOTO_FONT)
                    .color(Color::from_rgb(0.6, 0.6, 0.6))
            );
        } else {
            for request in &self.contact_requests {
                content = content.push(self.create_request_item(request));
            }
        }
        
        Scrollable::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_blocked_contacts(&self) -> Element<Message> {
        let mut content = Column::new().spacing(10);
        
        if self.blocked_contacts.is_empty() {
            content = content.push(
                Text::new("Nessun contatto bloccato")
                    .size(16)
                    .font(ROBOTO_FONT)
                    .color(Color::from_rgb(0.6, 0.6, 0.6))
            );
        } else {
            for contact in &self.blocked_contacts {
                content = content.push(self.create_contact_item(contact, true));
            }
        }
        
        Scrollable::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn create_contact_item<'a>(&self, contact: &'a Contact, is_blocked: bool) -> Element<'a, Message> {
        let avatar = Container::new(
            Text::new(contact.nickname.chars().next().unwrap_or('?').to_string())
                .size(20)
                .font(ROBOTO_FONT)
        )
        .width(Length::Fixed(45.0))
        .height(Length::Fixed(45.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.35))),
                border: Border::default().rounded(22.5),
                ..Default::default()
            }
        });
        
        let nickname = Text::new(&contact.nickname)
            .size(16)
            .font(ROBOTO_FONT);
        
        let peer_id = Text::new(&contact.peer_id)
            .size(12)
            .font(ROBOTO_FONT)
            .color(Color::from_rgb(0.7, 0.7, 0.7));
        
        let info = Column::new()
            .push(nickname)
            .push(peer_id)
            .spacing(2);
        
        let action_btn: Element<Message> = if is_blocked {
            Button::new(Text::new("Sblocca").size(12).font(ROBOTO_FONT))
                .padding([6, 12])
                .style(|_theme, status| {
                    let color = match status {
                        Status::Hovered => Color::from_rgb(0.3, 0.6, 0.3),
                        _ => Color::from_rgb(0.25, 0.55, 0.25),
                    };
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::WHITE,
                        border: Border::default().rounded(5),
                        shadow: Shadow::default(),
                        snap: false,
                    }
                })
                .on_press(Message::UnblockContact(contact.peer_id.clone()))
                .into()
        } else {
            Button::new(Text::new("Blocca").size(12).font(ROBOTO_FONT))
                .padding([6, 12])
                .style(|_theme, status| {
                    let color = match status {
                        Status::Hovered => Color::from_rgb(0.8, 0.3, 0.3),
                        _ => Color::from_rgb(0.7, 0.2, 0.2),
                    };
                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::WHITE,
                        border: Border::default().rounded(5),
                        shadow: Shadow::default(),
                        snap: false,
                    }
                })
                .on_press(Message::BlockContact(contact.peer_id.clone()))
                .into()
        };
        
        Container::new(
            Row::new()
                .push(avatar)
                .push(Space::new().width(Length::Fixed(15.0)))
                .push(info)
                .push(Space::new().width(Length::Fill))
                .push(action_btn)
                .align_y(Alignment::Center)
                .padding(10)
        )
        .width(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.17))),
                border: Border::default().rounded(8),
                ..Default::default()
            }
        })
        .into()
    }
    
    fn create_peer_search_result<'a>(&self, peer: &'a Contact) -> Element<'a, Message> {
        let avatar = Container::new(
            Text::new(peer.nickname.chars().next().unwrap_or('?').to_string())
                .size(20)
                .font(ROBOTO_FONT)
        )
        .width(Length::Fixed(45.0))
        .height(Length::Fixed(45.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.35))),
                border: Border::default().rounded(22.5),
                ..Default::default()
            }
        });
        
        let nickname = Text::new(&peer.nickname)
            .size(16)
            .font(ROBOTO_FONT);
        
        let peer_id = Text::new(&peer.peer_id)
            .size(12)
            .font(ROBOTO_FONT)
            .color(Color::from_rgb(0.7, 0.7, 0.7));
        
        let info = Column::new()
            .push(nickname)
            .push(peer_id)
            .spacing(2);
        
        let peer_id_clone = peer.peer_id.clone();
        let nickname_clone = peer.nickname.clone();
        
        let add_btn = Button::new(
            Text::new("👤+")
                .size(16)
                .font(ROBOTO_FONT)
                .shaping(text::Shaping::Advanced)
        )
        .padding([8, 12])
        .style(|_theme, status| {
            let color = match status {
                Status::Hovered => Color::from_rgb(0.25, 0.50, 0.95),
                _ => Color::from_rgb(0.20, 0.45, 0.90),
            };
            button::Style {
                background: Some(Background::Color(color)),
                text_color: Color::WHITE,
                border: Border::default().rounded(5),
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::SendContactRequest(peer_id_clone, nickname_clone));
        
        Container::new(
            Row::new()
                .push(avatar)
                .push(Space::new().width(Length::Fixed(15.0)))
                .push(info)
                .push(Space::new().width(Length::Fill))
                .push(add_btn)
                .align_y(Alignment::Center)
                .padding(10)
        )
        .width(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.17))),
                border: Border::default().rounded(8),
                ..Default::default()
            }
        })
        .into()
    }
    
    fn create_request_item<'a>(&self, request: &'a ContactRequest) -> Element<'a, Message> {
        let avatar = Container::new(
            Text::new(request.nickname.chars().next().unwrap_or('?').to_string())
                .size(20)
                .font(ROBOTO_FONT)
        )
        .width(Length::Fixed(45.0))
        .height(Length::Fixed(45.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.3, 0.3, 0.35))),
                border: Border::default().rounded(22.5),
                ..Default::default()
            }
        });
        
        let direction = if request.incoming { "→ Ricevuta" } else { "← Inviata" };
        
        let nickname = Text::new(format!("{} {}", &request.nickname, direction))
            .size(16)
            .font(ROBOTO_FONT);
        
        let peer_id = Text::new(&request.peer_id)
            .size(12)
            .font(ROBOTO_FONT)
            .color(Color::from_rgb(0.7, 0.7, 0.7));
        
        let info = Column::new()
            .push(nickname)
            .push(peer_id)
            .spacing(2);
        
        let peer_id_clone1 = request.peer_id.clone();
        let peer_id_clone2 = request.peer_id.clone();
        
        let actions = if request.incoming {
            Row::new()
                .push(
                    Button::new(Text::new("✓ Accetta").size(12).font(ROBOTO_FONT))
                        .padding([6, 12])
                        .style(|_theme, status| {
                            let color = match status {
                                Status::Hovered => Color::from_rgb(0.3, 0.6, 0.3),
                                _ => Color::from_rgb(0.25, 0.55, 0.25),
                            };
                            button::Style {
                                background: Some(Background::Color(color)),
                                text_color: Color::WHITE,
                                border: Border::default().rounded(5),
                                shadow: Shadow::default(),
                                snap: false,
                            }
                        })
                        .on_press(Message::AcceptContactRequest(peer_id_clone1))
                )
                .push(Space::new().width(Length::Fixed(8.0)))
                .push(
                    Button::new(Text::new("✗ Rifiuta").size(12).font(ROBOTO_FONT))
                        .padding([6, 12])
                        .style(|_theme, status| {
                            let color = match status {
                                Status::Hovered => Color::from_rgb(0.8, 0.3, 0.3),
                                _ => Color::from_rgb(0.7, 0.2, 0.2),
                            };
                            button::Style {
                                background: Some(Background::Color(color)),
                                text_color: Color::WHITE,
                                border: Border::default().rounded(5),
                                shadow: Shadow::default(),
                                snap: false,
                            }
                        })
                        .on_press(Message::RejectContactRequest(peer_id_clone2))
                )
        } else {
            Row::new()
                .push(
                    Text::new("In attesa...")
                        .size(12)
                        .font(ROBOTO_FONT)
                        .color(Color::from_rgb(0.6, 0.6, 0.6))
                )
        };
        
        Container::new(
            Row::new()
                .push(avatar)
                .push(Space::new().width(Length::Fixed(15.0)))
                .push(info)
                .push(Space::new().width(Length::Fill))
                .push(actions)
                .align_y(Alignment::Center)
                .padding(10)
        )
        .width(Length::Fill)
        .style(|_theme| {
            container::Style {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.17))),
                border: Border::default().rounded(8),
                ..Default::default()
            }
        })
        .into()
    }
    
    fn view_profile_tab(&self) -> Element<Message> {
        let title = Text::new("👤 Profilo")
            .size(32)
            .font(ROBOTO_FONT)
            .shaping(text::Shaping::Advanced);
        
        let user_info = if let Some(ref identity) = self.user_identity {
            format!(
                "Nickname: {}\n\
                Email: {}\n\
                ID: {}\n\
                Peer ID: {}",
                identity.nickname,
                identity.google_email,
                if identity.id.len() > 8 { &identity.id[..8] } else { &identity.id },
                if identity.peer_id.len() > 16 { &identity.peer_id[..16] } else { &identity.peer_id }
            )
        } else {
            "Identità non disponibile".to_string()
        };
        
        let info = Text::new(user_info)
            .size(16)
            .font(ROBOTO_FONT);
        
        let logout_btn = Button::new(
            Container::new(
                Text::new("🚪 Esci").size(15).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
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
                Status::Hovered => (Color::from_rgb(0.95, 0.3, 0.3), Shadow {
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 8.0,
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3)
                }),
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
        
        let layout = Column::new()
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(title)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(info)
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(logout_btn)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);
        
        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    
    fn view_settings_tab(&self) -> Element<Message> {
        let title = Text::new("⚙️ Impostazioni")
            .size(32)
            .font(ROBOTO_FONT)
            .shaping(text::Shaping::Advanced);
        
        let content = Text::new(
            "Configurazione applicazione\n\n\
            Qui potrai modificare:\n\
            • Preferenze account\n\
            • Impostazioni privacy\n\
            • Notifiche\n\
            • Temi e aspetto\n\
            • Rete e connessioni\n\n\
            In sviluppo..."
        )
        .size(16)
        .font(ROBOTO_FONT);
        
        let layout = Column::new()
            .push(Space::new().height(Length::Fixed(30.0)))
            .push(title)
            .push(Space::new().height(Length::Fixed(20.0)))
            .push(content)
            .push(Space::new().height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);
        
        Container::new(layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
