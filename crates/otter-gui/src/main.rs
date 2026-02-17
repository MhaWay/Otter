use iced::{
    widget::{Column, Container, Row, Scrollable, Text, Button, Space, TextInput, text},
    Element, Length, Sandbox, Settings, Theme, Font, Alignment,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const ROBOTO_FONT: Font = Font::with_name("Roboto");

// Source - https://stackoverflow.com/a/79782372
// Posted by Péter Szilvási, modified by community. See post 'Timeline' for change history
// Retrieved 2026-02-17, License - CC BY-SA 4.0

fn main() -> iced::Result {
    GuiApp::run(Settings {
        window: iced::window::Settings {
            exit_on_close_request: true,
            ..Default::default()
        },
        fonts: vec![
            include_bytes!("../fonts/Roboto-Regular.woff2")
                .as_slice()
                .into(),
        ],
        ..Default::default()
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Identity {
    pub id: String,
    pub nickname: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Clone, Debug)]
enum Message {
    TryLogin,
    StartRegister,
    DisclaimerScrolled(f32),
    DisclaimerAccepted,
    NicknameChanged(String),
    NicknameSubmit,
    BackToHome,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Disclaimer,
    ChooseNickname,
    MainApp,
}

struct GuiApp {
    current_screen: Screen,
    has_reached_bottom: bool,
    user_identity: Option<Identity>,
    nickname_input: String,
}

impl Default for GuiApp {
    fn default() -> Self {
        GuiApp {
            current_screen: Screen::Home,
            has_reached_bottom: false,
            user_identity: None,
            nickname_input: String::new(),
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

    fn create_new_identity(nickname: &str) -> Identity {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        
        Identity {
            id: uuid::Uuid::new_v4().to_string(),
            nickname: nickname.to_string(),
            public_key: "pk_placeholder".to_string(),
            created_at: timestamp,
        }
    }
}

impl Sandbox for GuiApp {
    type Message = Message;

    fn new() -> Self {
        let mut app = GuiApp::default();
        if let Some(identity) = GuiApp::load_identity() {
            app.user_identity = Some(identity);
            app.current_screen = Screen::MainApp;
        } else {
            app.current_screen = Screen::Home;
        }
        app
    }

    fn title(&self) -> String {
        String::from("Otter - Privacy-Focused Chat")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::TryLogin => {
                if let Some(identity) = GuiApp::load_identity() {
                    self.user_identity = Some(identity);
                    self.current_screen = Screen::MainApp;
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
                self.current_screen = Screen::ChooseNickname;
                self.nickname_input.clear();
            }
            Message::NicknameChanged(value) => {
                self.nickname_input = value;
            }
            Message::NicknameSubmit => {
                if !self.nickname_input.trim().is_empty() {
                    let identity = GuiApp::create_new_identity(&self.nickname_input);
                    if GuiApp::save_identity(&identity).is_ok() {
                        self.user_identity = Some(identity);
                        self.current_screen = Screen::MainApp;
                    }
                }
            }
            Message::BackToHome => {
                self.current_screen = Screen::Home;
                self.has_reached_bottom = false;
                self.nickname_input.clear();
            }
            Message::Logout => {
                let _ = GuiApp::delete_identity();
                self.user_identity = None;
                self.current_screen = Screen::Home;
                self.has_reached_bottom = false;
                self.nickname_input.clear();
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match self.current_screen {
            Screen::Home => self.view_home(),
            Screen::Disclaimer => self.view_disclaimer(),
            Screen::ChooseNickname => self.view_choose_nickname(),
            Screen::MainApp => self.view_main_app(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl GuiApp {
    fn view_home(&self) -> Element<Message> {
        let title = Text::new("🦦 Otter").size(64).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);

        let question = Text::new("Hai già un'identità?").size(22).font(ROBOTO_FONT);

        let login_btn = Button::new(
            Container::new(
                Text::new("📂 Accedi").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .width(Length::Fixed(280.0))
            .clip(true)
            .on_press(Message::TryLogin);

        let register_btn = Button::new(
            Container::new(
                Text::new("✨ Registrati").size(18).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .width(Length::Fixed(280.0))
            .clip(true)
            .on_press(Message::StartRegister);

        let content = Column::new()
            .push(Space::with_height(Length::Fill))
            .push(title)
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(question)
            .push(Space::with_height(Length::Fixed(30.0)))
            .push(
                Column::new()
                    .push(login_btn)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(register_btn)
                    .align_items(Alignment::Center)
            )
            .push(Space::with_height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center);

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
                .push(Space::with_height(Length::Fixed(15.0)))
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
                .center_x()
                .center_y()
                .width(Length::Fill)
                .height(Length::Fill)
            )
                .padding([8, 5])
                .width(Length::Fixed(320.0))
                .clip(true)
                .on_press(Message::DisclaimerAccepted)
                .into()
        } else {
            Button::new(
                Container::new(
                    Text::new(button_text).size(16).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
                )
                .center_x()
                .center_y()
                .width(Length::Fill)
                .height(Length::Fill)
            )
                .padding([8, 5])
                .width(Length::Fixed(320.0))
                .clip(true)
                .into()
        };

        let back_button: Element<Message> = Button::new(
            Container::new(
                Text::new("← Indietro").size(14).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .width(Length::Fixed(140.0))
            .clip(true)
            .on_press(Message::BackToHome)
            .into();

        let button_row = Row::new()
            .push(back_button)
            .push(Space::with_width(Length::Fill))
            .push(accept_button)
            .width(Length::Fill)
            .spacing(15);

        let content = Column::new()
            .push(disclaimer_scroll)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(button_row)
            .padding(30)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_choose_nickname(&self) -> Element<Message> {
        let title = Text::new("✨ Scegli il Tuo Nickname").size(40).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        let subtitle = Text::new("Questo sarà il nome con cui gli altri ti riconosceranno").size(16).font(ROBOTO_FONT);

        let input = TextInput::new("Inserisci il tuo nickname...", &self.nickname_input)
            .on_input(Message::NicknameChanged)
            .padding(12)
            .size(16)
            .width(Length::Fixed(350.0));

        let submit_button = Button::new(
            Container::new(
                Text::new("✓ Crea Identità").size(17).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .width(Length::Fixed(300.0))
            .clip(true)
            .on_press(Message::NicknameSubmit);

        let back_button = Button::new(
            Container::new(
                Text::new("← Indietro").size(14).font(ROBOTO_FONT).shaping(text::Shaping::Advanced)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .width(Length::Fixed(140.0))
            .clip(true)
            .on_press(Message::BackToHome);

        let button_row = Row::new()
            .push(back_button)
            .push(Space::with_width(Length::Fill))
            .push(submit_button)
            .width(Length::Fill)
            .spacing(15);

        let content = Column::new()
            .push(Space::with_height(Length::Fixed(80.0)))
            .push(title)
            .push(subtitle)
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(input)
            .push(Space::with_height(Length::Fixed(30.0)))
            .push(button_row)
            .push(Space::with_height(Length::Fill))
            .padding(40)
            .width(Length::Fill)
            .height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_main_app(&self) -> Element<Message> {
        let welcome = Text::new("🦦 Benvenuto su Otter!").size(48).font(ROBOTO_FONT).shaping(text::Shaping::Advanced);
        
        let user_info = if let Some(ref identity) = self.user_identity {
            Text::new(format!("Nickname: {}", identity.nickname)).size(20).font(ROBOTO_FONT)
        } else {
            Text::new("Identità: Sconosciuta").size(20).font(ROBOTO_FONT)
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
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
        )
            .padding([8, 5])
            .clip(true)
            .on_press(Message::Logout);

        let content = Column::new()
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(welcome)
            .push(user_info)
            .push(id_info)
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(status)
            .push(Space::with_height(Length::Fixed(30.0)))
            .push(placeholder)
            .push(Space::with_height(Length::Fill))
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
