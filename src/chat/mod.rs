pub(crate) mod controller;
mod ui;

pub(crate) use controller::ChatController;
pub(crate) use controller::GUIScrollText;
pub(crate) use controller::ChatScrollStopwatch;
pub(crate) use controller::CharacterSayMessage;
pub(crate) use controller::GUIChangeMessage;

const UI_Z_INDEX: i32 = 4;