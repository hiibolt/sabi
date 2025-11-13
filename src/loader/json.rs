use bevy::asset::AssetLoader;
use thiserror::Error;

use crate::character::CharacterConfig;

#[derive(Debug, Error)]
pub enum CharacterJsonError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Custom asset loader to parse characters configuration.
#[derive(Default)]
pub struct CharacterJsonLoader;
impl AssetLoader for CharacterJsonLoader {
    type Asset = CharacterConfig;
    type Settings = ();
    type Error = CharacterJsonError;

    fn load(
            &self,
            reader: &mut dyn bevy::asset::io::Reader,
            _settings: &Self::Settings,
            _load_context: &mut bevy::asset::LoadContext,
        ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let parsed: CharacterConfig = serde_json::from_slice(&bytes)?;
            Ok(parsed)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}