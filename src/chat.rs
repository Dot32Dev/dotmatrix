use eframe::egui;
use matrix_sdk::Client;
use matrix_sdk::Room;
use matrix_sdk::RoomState;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use tokio::sync::mpsc;

pub struct ChatApp {
    client: Client,
    event_receiver: mpsc::UnboundedReceiver<String>,
    messages: Vec<String>,
    ctx: egui::Context,
}

impl ChatApp {
    pub fn new(client: Client, ctx: egui::Context) -> ChatApp {
        let (event_send, event_rec) = mpsc::unbounded_channel();

        client.add_event_handler(
            move |event: OriginalSyncRoomMessageEvent, room: Room| async move {
                handle_room_message(event, room, event_send).await;
            },
        );

        let sync_client = client.clone();
        tokio::spawn(async move {
            let sync_settings = SyncSettings::default();
            if let Err(_) = sync_client.sync(sync_settings).await {
                return; // Silent fail, oh well
            }
        });

        Self {
            client,
            event_receiver: event_rec,
            messages: Vec::new(),
            ctx,
        }
    }

    pub fn draw(&mut self) {
        if let Ok(event) = self.event_receiver.try_recv() {
            self.messages.push(event);
        }

        egui::CentralPanel::default().show(&self.ctx.clone(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT)
                        .with_cross_justify(true),
                    |ui| {
                        for message in &self.messages {
                            ui.label(message);
                        }
                    },
                );
            });
        });
    }
}

async fn handle_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    sender: mpsc::UnboundedSender<String>,
) {
    if room.state() != RoomState::Joined {
        return;
    }

    // We only want to log text messages.
    let MessageType::Text(msgtype) = &event.content.msgtype else {
        return;
    };

    let member = room
        .get_member(&event.sender)
        .await
        .expect("Couldn't get the room member")
        .expect("The room member doesn't exist");
    let name = member.name();

    let room_name = if let Some(maybe_name) = room.canonical_alias() {
        maybe_name.to_string()
    } else {
        String::new()
    };

    _ = sender.send(format!("{room_name} -> {name}: {}", msgtype.body));
}
