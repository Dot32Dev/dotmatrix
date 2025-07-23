use eframe::egui;
use egui::{Color32, Id, Modal, RichText};
use matrix_sdk::{
    Client, Room, RoomState,
    config::SyncSettings,
    ruma::{
        api::client::session::{
            get_login_types::v3::{IdentityProvider, LoginType},
            login::v3::HomeserverInfo,
        },
        events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
    },
};
use tokio::sync::mpsc;
use url::Url;

mod password_widgit;

const DEFAULT_DEVICE_NAME: &str = "Dot32's Matrix Client";

#[tokio::main]
async fn main() {
    // Logging to stdout for matrix
    tracing_subscriber::fmt::init();

    // Egui window options
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Dotmatrix",
        native_options,
        Box::new(|cc| Ok(Box::new(EguiApp::new(cc)))),
    )
    .unwrap();
}

#[derive(Default)]
enum HomeserverStatus {
    #[default]
    Idle,
    Connecting,
    GettingAuthTypes,
    AuthTypes(Vec<LoginChoice>),
    Error(String),
}

// Taken from matrix sdk login example
#[derive(Debug)]
enum LoginChoice {
    // Login with username and password.
    Password,
    // Login with SSO.
    Sso,
    // Login with a specific SSO identity provider.
    SsoIdp(IdentityProvider),
}

// #[derive(Default)]
// enum LoginStatus {
//     #[default]
//     Idle,
//     LoggingIn,
//     Success,
//     Error(String),
// }

#[derive(Default)]
struct EguiApp {
    homeserver: String,
    localpart: String,
    password: String,
    // Channels
    homeserver_status: HomeserverStatus,
    homeserver_receiver: Option<mpsc::UnboundedReceiver<HomeserverStatus>>,
    // login_status: LoginStatus,
    // login_receiver: Option<mpsc::UnboundedReceiver<LoginStatus>>,
}

impl EguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut egui_app = Self::default();
        egui_app.homeserver = "matrix.org".to_string();

        egui_app.homeserver_connect(&cc.egui_ctx);

        egui_app
    }

    fn homeserver_connect(&mut self, ctx: &egui::Context) {
        let homeserver = format!("https://{}", self.homeserver);

        let (sender, receiver) = mpsc::unbounded_channel();
        self.homeserver_receiver = Some(receiver);
        self.homeserver_status = HomeserverStatus::Connecting;

        let ctx = ctx.clone();

        tokio::spawn(async move {
            let Ok(homeserver_url) = Url::parse(&homeserver) else {
                let _ = sender.send(HomeserverStatus::Error(
                    "Failed to parse URL".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            let Ok(client) = Client::new(homeserver_url).await else {
                let _ = sender.send(HomeserverStatus::Error(
                    "Could not find home server".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            let _ = sender.send(HomeserverStatus::GettingAuthTypes);
            ctx.request_repaint();

            let mut choices = Vec::new();
            let Ok(login_types) = client.matrix_auth().get_login_types().await
            else {
                let _ = sender.send(HomeserverStatus::Error(
                    "Could not get auth options".to_string(),
                ));
                ctx.request_repaint();
                return;
            };

            for login_type in login_types.flows {
                match login_type {
                    LoginType::Password(_) => {
                        choices.push(LoginChoice::Password)
                    }
                    LoginType::Sso(sso) => {
                        if sso.identity_providers.is_empty() {
                            choices.push(LoginChoice::Sso)
                        } else {
                            choices.extend(
                                sso.identity_providers
                                    .into_iter().map(LoginChoice::SsoIdp))
                        }
                    }
                    // This is used for SSO, so it's not a separate choice.
                    LoginType::Token(_) |
                    // This is only for application services, ignore it here.
                    LoginType::ApplicationService(_) => {},
                    // We don't support unknown login types.
                    _ => {},
                }
            }

            let _ = sender.send(HomeserverStatus::AuthTypes(choices));
        });
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Modal::new(Id::new("Login")).show(ui.ctx(), |ui| {
                ui.set_width(250.0);
                ui.set_min_height(200.0);
                ui.vertical_centered(|ui| {
                    ui.heading("Login");

                    ui.add_enabled_ui(!false, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Homeserver:");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.homeserver)
                                    .hint_text("Homeserver")
                                    .id(egui::Id::new("homeserver_input"))
                            );

                            if ui.button("Search").clicked()
                                || (response.lost_focus()
                                    && ui
                                        .input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                match self.homeserver_status {
                                    HomeserverStatus::Idle | HomeserverStatus::AuthTypes(_) => {
                                        self.homeserver_connect(ctx);
                                    }
                                    _ => ()
                                }
                            }
                        });

                        if let Some(receiver) = &mut self.homeserver_receiver {
                            if let Ok(status) = receiver.try_recv() {
                                self.homeserver_status = status;
                                match self.homeserver_status {
                                    HomeserverStatus::AuthTypes(_) => {
                                        self.homeserver_receiver = None;
                                    }
                                    _ => (),
                                }
                            }
                        }

                        match self.homeserver_status {
                            HomeserverStatus::Idle => (),
                            HomeserverStatus::Connecting => {
                                ui.horizontal(|ui| {
                                    ui.label("Connecting to home server");
                                    ui.spinner();
                                });
                            }
                            HomeserverStatus::GettingAuthTypes => {
                                ui.horizontal(|ui| {
                                    ui.label("Fetching authentication options");
                                    ui.spinner();
                                });
                            }
                            HomeserverStatus::Error(ref error) => {
                                ui.label(
                                    RichText::new(error)
                                        .color(Color32::from_rgb(220, 30, 30)),
                                );
                            }
                            HomeserverStatus::AuthTypes(ref auth_types) => {
                                ui.separator();
                                for (i, login_choice) in
                                    auth_types.iter().enumerate()
                                {
                                    match login_choice {
                                        LoginChoice::Password => {
                                            ui.label("Login with password:");
                                            ui.horizontal(|ui| {
                                                ui.label("Username:");
                                                ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.localpart,
                                                    )
                                                    .hint_text("Username"),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Password:");
                                                ui.add(password_widgit::password(
                                                    &mut self.password,
                                                ));
                                            });

                                            if ui.button("Login").clicked() {
                                                // self.button_clicked = true;
                                            }
                                        }
                                        LoginChoice::Sso => {
                                            ui.label("Login with SSO:");
                                            if ui
                                                .button("Open in browser")
                                                .clicked()
                                            {
                                            }
                                        }
                                        LoginChoice::SsoIdp(_idp) => {
                                            ui.label("Login with SSO and identify provider:");
                                            if ui
                                                .button("Open in browser")
                                                .clicked()
                                            {
                                            }
                                        }
                                    }
                                    if i != auth_types.len() - 1 {
                                        ui.separator();
                                    }
                                }
                            }
                        }
                    });
                });
            });
        });
    }
}

// use matrix_sdk::{
//     Client,
//     config::SyncSettings,
//     ruma::{events::room::message::SyncRoomMessageEvent, user_id},
// };

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let user = user_id!("@dot32:matrix.tturna.com");
//     let client = Client::builder()
//         .server_name(user.server_name())
//         .build()
//         .await?;

//     // First we need to log in.
//     client
//         .matrix_auth()
//         .login_username(user, "")
//         .send()
//         .await?;

//     client.add_event_handler(|ev: SyncRoomMessageEvent| async move {
//         println!("Received a message {:?}", ev);
//         println!();
//     });

//     // Syncing is important to synchronize the client state with the server.
//     // This method will never return unless there is an error.
//     client.sync(SyncSettings::default()).await?;

//     Ok(())
// }
