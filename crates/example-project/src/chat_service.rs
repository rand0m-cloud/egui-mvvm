use egui_mvvm::ref_state::RefState;
use egui_mvvm::val_state::ValState;
use egui_mvvm::view_model;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ChannelId(pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ChatMessageId(pub ChannelId, pub usize);

pub struct ChatMessage {
    pub author: String,
    pub message: String,
    pub timestamp: jiff::Timestamp,
}

#[derive(Default)]
pub struct Channel {
    name: Arc<str>,
    messages: Vec<ChatMessageId>,
}

view_model! {
    #[viewmodel(default)]
    pub struct ChatService {
        last_channel_id: ValState<ChannelId> = ChannelId(0),
        last_msg_id: ValState<usize> = 0,
        messages: RefState<HashMap<ChatMessageId, ChatMessage>> = HashMap::new(),
        channels: RefState<HashMap<ChannelId, Channel>> = HashMap::new(),
    }
}

impl ChatService {
    pub fn channel_name(&self, id: ChannelId) -> Option<Arc<str>> {
        self.channels.value().get(&id).map(|c| c.name.clone())
    }

    pub fn channel_message_ids(&self, id: ChannelId, mut walk: impl FnMut(&Vec<ChatMessageId>)) {
        self.channels.value().get(&id).map(|c| walk(&c.messages));
    }
    pub fn channel_messages(
        &self,
        channel_id: ChannelId,
        mut walk: impl FnMut(ChatMessageId, &ChatMessage),
    ) {
        self.channel_message_ids(channel_id, |ids| {
            for id in ids {
                self.message(*id, |msg| walk(*id, msg))
            }
        });
    }

    pub fn message(&self, chat_message_id: ChatMessageId, walk: impl FnOnce(&ChatMessage)) {
        self.messages.value().get(&chat_message_id).map(walk);
    }

    pub fn send_message(&self, channel_id: ChannelId, message: ChatMessage) {
        let id = self.last_msg_id.latest_value() + 1;
        self.last_msg_id.send_modify(|id| *id = *id + 1);

        let chat_message_id = ChatMessageId(channel_id, id);

        self.messages.send_modify(|msgs| {
            msgs.insert(chat_message_id, message);
        });

        self.channels.send_modify(|channels| {
            channels
                .entry(channel_id)
                .or_default()
                .messages
                .push(chat_message_id)
        })
    }

    pub fn create_channel(&self, name: &str) -> ChannelId {
        let id = self.last_channel_id.latest_value().0 + 1;
        self.last_channel_id.send_modify(|id| id.0 = id.0 + 1);

        let channel_id = ChannelId(id);

        self.channels
            .send_modify(|channels| channels.entry(channel_id).or_default().name = name.into());

        channel_id
    }

    pub fn channels(&self, mut walk: impl FnMut(ChannelId)) {
        for id in self.channels.value().keys() {
            walk(*id);
        }
    }
}
