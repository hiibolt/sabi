pub(crate) mod controller;
mod ui;

pub(crate) use controller::ChatController;
pub(crate) use controller::GUIScrollText;
pub(crate) use controller::CharacterSayMessage;
pub(crate) use controller::GUIChangeMessage;

const INFOTEXT_Z_INDEX_ACTIVE: i32 = 4;
const INFOTEXT_Z_INDEX_INACTIVE: i32 = -1;
const UI_Z_INDEX: i32 = 5;