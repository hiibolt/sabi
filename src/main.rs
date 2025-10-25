mod background;
mod character;
mod chat;
mod compiler;

use crate::background::*;
use crate::character::*;
use crate::chat::*;
use crate::compiler::*;
use crate::compiler::ast;

use bevy::asset::AssetLoader;
use bevy::ecs::error::ErrorContext;
use bevy::{
    prelude::*,
    window::*,
};
use std::vec::IntoIter;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CharacterJsonError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Serde(#[from] serde_json::Error),
}

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

#[derive(Resource, Default)]
pub struct VisualNovelState {
    // Player-designated constants
    playername: String,

    // Game state
    acts: ast::Acts,
    act: Box<ast::Act>,
    scene: Box<ast::Scene>,
    statements: IntoIter<ast::Statement>,
    blocking: bool,
}

fn error_handler ( err: BevyError, ctx: ErrorContext ) {
    panic!("Bevy error: {err:?}\nContext: {ctx:?}")
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Sabi"),
                    resolution: (1280, 800).into(),
                    present_mode: PresentMode::AutoVsync,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
                })
        )
        .init_resource::<VisualNovelState>()
        .init_asset::<CharacterConfig>()
        .init_asset_loader::<CharacterJsonLoader>()
        .set_error_handler(error_handler)
        .add_systems(Startup, setup)
        .add_plugins((
            Compiler,
            BackgroundController,
            CharacterController,
            ChatController,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut game_state: ResMut<VisualNovelState>,
) {
    // This would normally be filled in by the player
    game_state.playername = String::from("Bolt");

    // Create our primary camera (which is
    //  necessary even for 2D games)
    commands.spawn(Camera2d::default());
}
