use crate::password_widgit;
use eframe::egui;
use egui::Color32;
use egui::Id;
use egui::Modal;
use egui::RichText;
use matrix_sdk::Client;
use matrix_sdk::ruma::api::client::session::get_login_types::v3::LoginType;
use tokio::sync::mpsc;
use url::Url;
// use egui::{Color32, Id, Modal, RichText};
// use matrix_sdk::{
//     Client, Room, RoomState,
//     config::SyncSettings,
//     ruma::{
//         api::client::session::{
//             get_login_types::v3::{IdentityProvider, LoginType},
//             login::v3::HomeserverInfo,
//         },
//         events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
//     },
// };

const DEFAULT_DEVICE_NAME: &str = "Dotmatrix";
const MODAL_WIDTH: f32 = 350.0;
const MODAL_HEIGHT: f32 = 370.0;
const LABEL_WIDTH: f32 = 75.0;
const TEXTBOX_WIDTH: f32 = 200.0;
const TEXTBOX_HEIGHT: f32 = 32.0;
const TEXTBOX_MARGIN_X: f32 = 12.0;
const TEXTBOX_MARGIN_Y: f32 = 8.0;

enum LoginMethod {
    Password,
    Sso,
}

#[derive(Default)]
enum HomeserverState {
    #[default]
    Idle,
    GettingAuthTypes,
    AuthTypes(Vec<LoginMethod>),
    Error(String),
}

#[derive(Default)]
enum LoginState {
    #[default]
    Idle,
    WaitingForUser,
    LoggingIn,
    Success,
    Error(String),
}

#[derive(Default)]
pub struct LoginApp {
    homeserver: String,
    localpart: String,
    password: String,
    ctx: egui::Context,

    homeserver_status: HomeserverState,
    homeserver_receiver: Option<mpsc::UnboundedReceiver<HomeserverState>>,

    login_status: LoginState,
    login_receiver: Option<mpsc::UnboundedReceiver<LoginState>>,

    client: Option<Client>,
    client_reciever: Option<mpsc::UnboundedReceiver<Client>>,
}

impl LoginApp {
    pub fn new(ctx: egui::Context) -> Self {
        let mut app = Self::default();
        app.ctx = ctx;
        app.homeserver = "matrix.org".to_string();
        app.get_auth_methods();
        app
    }

    fn get_auth_methods(&mut self) {
        let homeserver = format!("https://{}", self.homeserver);

        let (auth_send, auth_rec) = mpsc::unbounded_channel();
        self.homeserver_receiver = Some(auth_rec);

        let (client_send, client_rec) = mpsc::unbounded_channel();
        self.client_reciever = Some(client_rec);
        self.client = None;

        let ctx = self.ctx.clone();

        tokio::spawn(async move {
            let Ok(homeserver_url) = Url::parse(&homeserver) else {
                let _ = auth_send.send(HomeserverState::Error(
                    "Failed to parse URL".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            let Ok(client) = Client::new(homeserver_url).await else {
                let _ = auth_send.send(HomeserverState::Error(
                    "Could not find home server".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            let _ = auth_send.send(HomeserverState::GettingAuthTypes);
            ctx.request_repaint();

            let mut choices = Vec::new();
            let Ok(login_types) = client.matrix_auth().get_login_types().await
            else {
                let _ = auth_send.send(HomeserverState::Error(
                    "Could not get auth options".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            for login_type in login_types.flows {
                match login_type {
                    LoginType::Password(_) => {
                        choices.push(LoginMethod::Password)
                    }
                    LoginType::Sso(sso) => {
                        if sso.identity_providers.is_empty() {
                            choices.push(LoginMethod::Sso)
                        }
                    }
                    _ => {}
                }
            }

            let _ = auth_send.send(HomeserverState::AuthTypes(choices));
            let _ = client_send.send(client);
            ctx.request_repaint();
        });
    }

    fn password_login(&mut self) {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.login_receiver = Some(receiver);
        self.login_status = LoginState::LoggingIn;
        self.ctx.request_repaint();

        let client = self
            .client
            .as_ref()
            .expect(
                "The client is created when the login options are presented",
                // How would you login without the login options being shown?
            )
            .clone();

        let username = self.localpart.clone();
        let password = self.password.clone();

        let ctx = self.ctx.clone();

        tokio::spawn(async move {
            match client
                .matrix_auth()
                .login_username(&username, &password)
                .initial_device_display_name(DEFAULT_DEVICE_NAME)
                .await
            {
                Ok(_) => {
                    _ = sender.send(LoginState::Success);
                    ctx.request_repaint();
                }
                Err(error) => {
                    _ = sender.send(LoginState::Error(error.to_string()));
                    ctx.request_repaint();
                }
            }
        });
    }

    fn sso_login(&mut self) {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.login_receiver = Some(receiver);
        self.login_status = LoginState::LoggingIn;
        self.ctx.request_repaint();

        let client = self
            .client
            .as_ref()
            .expect(
                "The client is created when the login options are presented",
                // How would you login without the login options being shown?
            )
            .clone();

        let ctx = self.ctx.clone();

        tokio::spawn(async move {
            _ = sender.send(LoginState::WaitingForUser);
            ctx.request_repaint();
            let login_builder =
                client.matrix_auth().login_sso(|url| async move {
                    open::that(url)?;
                    Ok(())
                });

            match login_builder.await {
                Ok(_) => {
                    _ = sender.send(LoginState::Success);
                    ctx.request_repaint();
                }
                Err(error) => {
                    _ = sender.send(LoginState::Error(error.to_string()));
                    ctx.request_repaint();
                }
            }
        });
    }

    fn update_recievers(&mut self) {
        if let Some(receiver) = &mut self.homeserver_receiver {
            if let Ok(status) = receiver.try_recv() {
                self.homeserver_status = status;
                match self.homeserver_status {
                    HomeserverState::AuthTypes(_) => {
                        self.homeserver_receiver = None;
                    }
                    _ => (),
                }
            }
        }

        if let Some(receiver) = &mut self.client_reciever {
            if let Ok(status) = receiver.try_recv() {
                self.client = Some(status);
                self.client_reciever = None;
            }
        }

        if let Some(receiver) = &mut self.login_receiver {
            if let Ok(status) = receiver.try_recv() {
                self.login_status = status;
                match self.login_status {
                    LoginState::Success | LoginState::Error(_) => {
                        self.login_receiver = None;
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn draw(&mut self) {
        self.update_recievers();

        egui::CentralPanel::default().show(&self.ctx.clone(), |ui| {
            Modal::new(Id::new("Login")).show(ui.ctx(), |ui| {
                ui.set_width(MODAL_WIDTH);
                ui.set_min_height(MODAL_HEIGHT);
                ui.vertical_centered(|ui| {
                    ui.heading("Login");

                    let enabled =
                        !matches!(self.login_status, LoginState::LoggingIn);

                    ui.add_enabled_ui(enabled, |ui| {
                        ui.horizontal(|ui| {
                            centered_label(ui, "Homeserver:");

                            let response = centered_textbox(
                                ui,
                                "Homeserver",
                                &mut self.homeserver,
                            );

                            if ui
                                .button("🔍")
                                .on_hover_text("Search for homeserver")
                                .clicked()
                                || (response.lost_focus()
                                    && ui.input(|i| {
                                        i.key_pressed(egui::Key::Enter)
                                    }))
                            {
                                match self.homeserver_status {
                                    HomeserverState::Idle
                                    | HomeserverState::AuthTypes(_)
                                    | HomeserverState::Error(_) => {
                                        self.get_auth_methods();
                                    }
                                    _ => (),
                                }
                            }
                        });

                        ui.separator();

                        match self.homeserver_status {
                            HomeserverState::Idle => (),
                            HomeserverState::GettingAuthTypes => {
                                ui.horizontal(|ui| {
                                    ui.label("Fetching authentication options");
                                    ui.spinner();
                                });
                            }
                            HomeserverState::Error(ref error) => {
                                ui.label(
                                    RichText::new(error)
                                        .color(Color32::from_rgb(220, 30, 30)),
                                );
                            }
                            HomeserverState::AuthTypes(_) => {
                                self.draw_login_options(ui);
                            }
                        }

                        self.draw_login_status(ui);
                    });
                });
            });
        });
    }

    fn draw_login_status(&mut self, ui: &mut egui::Ui) {
        match self.login_status {
            LoginState::Idle => {}
            LoginState::WaitingForUser => {
                ui.horizontal(|ui| {
                    ui.label("Waiting for user verification");
                    ui.spinner();
                });
            }
            LoginState::LoggingIn => {
                ui.horizontal(|ui| {
                    ui.label("Loggin in");
                    ui.spinner();
                });
            }
            LoginState::Success => {
                ui.label("Logged in");
            }
            LoginState::Error(ref error) => {
                ui.label(
                    RichText::new(error).color(Color32::from_rgb(220, 30, 30)),
                );
            }
        }
    }

    fn draw_login_options(&mut self, ui: &mut egui::Ui) {
        let HomeserverState::AuthTypes(ref auth_types) = self.homeserver_status
        else {
            return;
        };

        let mut password_login_clicked = false;
        let mut sso_login_clicked = false;

        for (i, login_choice) in auth_types.iter().enumerate() {
            match login_choice {
                LoginMethod::Password => {
                    ui.label("Login with password:");
                    ui.horizontal(|ui| {
                        centered_label(ui, "Username:");
                        centered_textbox(ui, "Username", &mut self.localpart);
                    });

                    ui.horizontal(|ui| {
                        centered_label(ui, "Password:");
                        ui.add(password_widgit::password(&mut self.password));
                    });

                    if ui.button("Login").clicked() {
                        password_login_clicked = true; //mf
                    }
                }
                LoginMethod::Sso => {
                    ui.label("Login with SSO:");
                    if ui.button("Open in browser").clicked() {
                        sso_login_clicked = true;
                    }
                }
            }
            if i != auth_types.len() - 1 {
                ui.separator();
            }
        }

        // I fucking hate Rust for necessitating this
        // Borrow checker really won't let me call these inside the loop
        if password_login_clicked {
            self.password_login();
        }
        if sso_login_clicked {
            self.sso_login();
        }
    }

    pub fn ready(&self) -> bool {
        return matches!(self.login_status, LoginState::Success);
    }

    pub fn take_client(&mut self) -> Option<Client> {
        return std::mem::take(&mut self.client);
    }
}

fn centered_label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    return ui
        .allocate_ui_with_layout(
            egui::vec2(LABEL_WIDTH, TEXTBOX_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_sized(
                    [LABEL_WIDTH, TEXTBOX_HEIGHT],
                    egui::Label::new(text),
                );
            },
        )
        .response;
}

fn centered_textbox(
    ui: &mut egui::Ui,
    hint_text: &str,
    text: &mut String,
) -> egui::Response {
    ui.add_sized(
        [TEXTBOX_WIDTH, TEXTBOX_HEIGHT],
        egui::TextEdit::singleline(text)
            .margin(egui::vec2(TEXTBOX_MARGIN_X, TEXTBOX_MARGIN_Y))
            .hint_text(hint_text),
    )
}
