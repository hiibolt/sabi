mod background;
mod character;
mod chat;
mod compiler;
mod loader;

use crate::background::*;
use crate::character::*;
use crate::chat::*;
use crate::compiler::*;
use crate::loader::CharacterJsonLoader;
use crate::loader::PestLoader;

use bevy::prelude::*;
use std::vec::IntoIter;
use bevy::ecs::error::ErrorContext;

/// Resource containing main [Act] state and related runtime data for the Visual Novel.
/// Player-designated constants are passe by the [UserDefinedConstants] resource.
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

fn sabi_error_handler ( err: BevyError, ctx: ErrorContext ) {
    panic!("Bevy error: {err:?}\nContext: {ctx:?}")
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ScriptId {
    pub chapter: String,
    pub act: String,
}

#[derive(Message)]
pub struct SabiStart(pub ScriptId);

pub struct SabiPlugin;
impl Plugin for SabiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserDefinedConstants>()
            .init_resource::<VisualNovelState>()
            .init_asset::<CharacterConfig>()
            .init_asset_loader::<CharacterJsonLoader>()
            .init_asset::<ast::Act>()
            .init_asset_loader::<PestLoader>()
            .set_error_handler(sabi_error_handler)
            .add_plugins((
                Compiler,
                BackgroundController,
                CharacterController,
                ChatController
            ));
    }
    
}