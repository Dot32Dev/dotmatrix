use eframe::egui;

// Password wigit from demo:
// https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/password.rs

fn password_ui(ui: &mut egui::Ui, password: &mut String) -> egui::Response {
    let state_id = ui.id().with("show_plaintext");

    let mut show_plaintext =
        ui.data_mut(|d| d.get_temp::<bool>(state_id).unwrap_or(false));

    let result = ui.with_layout(
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let response = ui
                .selectable_label(show_plaintext, "👁")
                .on_hover_text("Show/hide password");

            if response.clicked() {
                show_plaintext = !show_plaintext;
            }

            ui.add_sized(
                ui.available_size(),
                egui::TextEdit::singleline(password)
                    .password(!show_plaintext)
                    .hint_text("Password"),
            );
        },
    );

    ui.data_mut(|d| d.insert_temp(state_id, show_plaintext));

    result.response
}
pub fn password(password: &mut String) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| password_ui(ui, password)
}
