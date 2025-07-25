use eframe::egui;
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
// use tokio::sync::mpsc;

mod login;
mod password_widgit;

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

enum State {
    Login(login::LoginApp),
    Chat,
}

struct EguiApp(State);

impl EguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style: egui::Style = (*cc.egui_ctx.style()).clone();

        style.spacing.item_spacing = egui::vec2(12.0, 12.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);

        cc.egui_ctx.set_style(style);

        let egui_app =
            EguiApp(State::Login(login::LoginApp::new(cc.egui_ctx.clone())));

        egui_app
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match &mut self.0 {
            State::Login(login_app) => {
                login_app.draw();

                if login_app.ready() {
                    self.0 = State::Chat;
                }
            }
            State::Chat => (),
        }
    }
}
