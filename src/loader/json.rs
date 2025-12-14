use bevy::asset::AssetLoader;
use thiserror::Error;

use crate::character::{CharacterConfig, controller::{ActorConfig, AnimationConfig}};

#[derive(Debug, Error)]
pub enum ActorJsonError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Custom asset loader to parse characters configuration.
#[derive(Default)]
pub struct ActorJsonLoader;
impl AssetLoader for ActorJsonLoader {
    type Asset = ActorConfig;
    type Settings = ();
    type Error = ActorJsonError;

    fn load(
            &self,
            reader: &mut dyn bevy::asset::io::Reader,
            _settings: &Self::Settings,
            _load_context: &mut bevy::asset::LoadContext,
        ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            if let Ok(parsed) = serde_json::from_slice::<CharacterConfig>(&bytes) {
                Ok(ActorConfig::Character(parsed))
            } else {
                let parsed = serde_json::from_slice::<AnimationConfig>(&bytes)?;
                Ok(ActorConfig::Animation(parsed.clone()))
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}