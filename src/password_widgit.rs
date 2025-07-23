use eframe::egui;

// Password wigit from demo:
// https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/password.rs

fn password_ui(ui: &mut egui::Ui, password: &mut String) -> egui::Response {
    let state_id = ui.id().with("show_plaintext");

    let mut show_plaintext =
        ui.data_mut(|d| d.get_temp::<bool>(state_id).unwrap_or(false));

    let result = ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_sized(
                [200., 32.],
                egui::TextEdit::singleline(password)
                    .margin(egui::vec2(12.0, 8.0))
                    .password(!show_plaintext)
                    .hint_text("Password"),
            );

            let response = ui.button("👁").on_hover_text("Show/hide password");

            if response.clicked() {
                show_plaintext = !show_plaintext;
            }
        },
    );

    ui.data_mut(|d| d.insert_temp(state_id, show_plaintext));

    result.response
}
pub fn password(password: &mut String) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| password_ui(ui, password)
}
