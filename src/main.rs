mod background;
mod character;
mod chat;
mod compiler;
mod loader;

use crate::background::*;
use crate::character::*;
use crate::chat::*;
use crate::compiler::ast::Act;
use crate::compiler::controller::SabiStart;
use crate::compiler::controller::ScriptId;
use crate::compiler::*;
use crate::compiler::ast;
use crate::loader::CharacterJsonLoader;
use crate::loader::PestLoader;

use bevy::ecs::error::ErrorContext;
use bevy::{
    prelude::*,
    window::*,
};
use std::vec::IntoIter;

/// Resource containing main configuration of Visual Novel.\n
/// It mainly handles [Act] state and player-designated constants
#[derive(Resource, Default)]
pub(crate) struct VisualNovelState {
    // Player-designated constants
    playername: String,

    act: Box<ast::Act>,
    scene: Box<ast::Scene>,
    statements: IntoIter<ast::Statement>,
    blocking: bool,
}

#[derive(Resource, Default)]
pub struct UserDefinedConstants {
    pub playername: String,
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
        .init_resource::<UserDefinedConstants>()
        .init_resource::<VisualNovelState>()
        .init_asset::<CharacterConfig>()
        .init_asset_loader::<CharacterJsonLoader>()
        .init_asset::<Act>()
        .init_asset_loader::<PestLoader>()
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
    mut msg_writer: MessageWriter<SabiStart>,
    user_defined_constants: Res<UserDefinedConstants>,
) {
    // This would normally be filled in by the player
    game_state.playername = user_defined_constants.playername.clone();

    // Create our primary camera (which is
    //  necessary even for 2D games)
    commands.spawn(Camera2d::default());
    msg_writer.write(SabiStart(ScriptId { chapter: "Chapter 1".into(), act: "1".into() }));
}
