mod background;
mod character;
mod chat;
mod compiler;
mod loader;

use crate::background::*;
use crate::character::*;
use crate::chat::*;
use crate::compiler::ast::Statement;
use crate::compiler::*;
use crate::loader::CharacterJsonLoader;
use crate::loader::PestLoader;

use anyhow::Context;
use bevy::prelude::*;
use bevy::ecs::error::ErrorContext;

/// Resource containing main [Act] state and related runtime data for the Visual Novel.
/// Player-designated constants are passe by the [UserDefinedConstants] resource.
#[derive(Resource, Default)]
pub(crate) struct VisualNovelState {
    // Player-designated constants
    playername: String,

    pub act: Box<Act>,
    blocking: bool,
    pub rewinding: usize,
}

impl VisualNovelState {
    fn set_rewind(&mut self) {
        if let Ok(steps) = self.act.rewind_steps() {
            self.rewinding = steps;
            self.blocking = false;
        }
    }
}

#[derive(Debug, Clone, Default, Asset, TypePath)]
pub(crate) struct Act {
    pub scenes_reader: ScenesReader,
    pub name: String,
    pub history: Vec<Statement>,
}

impl Act {
    pub(crate) fn contains_scene(&self, scene_id: &str) -> bool {
        self.scenes_reader.scenes.iter().find(|s| s.name == scene_id).is_some()
    }
    
    pub(crate) fn rewind_steps(&mut self) -> Result<usize, BevyError> {
        self.scenes_reader.rewind_steps()
    }
    
    pub(crate) fn rewind(&mut self) {
        self.scenes_reader.rewind();
    }
    
    pub(crate) fn advance(&mut self) {
        let _ = self.scenes_reader.advance();
    }
    
    pub(crate) fn current(&self) -> Option<ast::Statement> {
        self.scenes_reader.current_statement()
    }
    
    pub(crate) fn add_scene(&mut self, scene: ast::Scene) {
        self.scenes_reader.add_scene(scene);
    }
    
    pub(crate) fn change_scene(&mut self, scene_id: &str) -> Result<(), BevyError> {
        self.scenes_reader.change_scene(scene_id)
    }
    
    pub(crate) fn history(&self) -> Vec<Statement> {
        self.history.clone()
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct ScenesReader {
    pub scenes: Vec<ast::Scene>,
    index: usize,
}

impl ScenesReader {
   fn advance(&mut self) -> Result<(), BevyError> {
        match self.current_mut() {
            Some(scene) => {
                if scene.statements_reader.advance().is_err() {
                    if self.index + 1 < self.scenes.len() {
                        self.index += 1;
                        let scene = self.scenes.get_mut(self.index).context("No scene found")?;
                        scene.statements_reader.index = 0;
                        Ok(())
                    } else {
                        Err(BevyError::from("Act's scenes are finished"))
                    }
                } else { Ok(()) }
            },
            _ => Err(BevyError::from("No scene is available"))
        }
    }
    
    pub(crate) fn rewind_steps(&mut self) -> Result<usize, BevyError> {
        match &mut self.current() {
            Some(scene) => {
                // TODO(rewind): if at its first statement,
                // scene needs to go backwards if possible
                scene.statements_reader.rewind_steps()
            },
            _ => { Err(BevyError::from("No scene available!")) }
        }
    }
    
    pub(crate) fn rewind(&mut self) {
        if let Some(scene) = self.current_mut() {
            scene.statements_reader.rewind();
        }
    }
    
    pub(crate) fn current(&self) -> Option<&ast::Scene> {
        self.scenes.get(self.index)
    }
    
    fn current_mut(&mut self) -> Option<&mut ast::Scene> {
        self.scenes.get_mut(self.index)
    }
    
    pub(crate) fn current_statement(&self) -> Option<ast::Statement> {
        match self.current() {
            Some(scene) => scene.statements_reader.current(),
            None => None
        }
    }
    
    pub(crate) fn add_scene(&mut self, scene: ast::Scene) {
        self.scenes.push(scene);
    }
    
    pub(crate) fn change_scene(&mut self, scene_id: &str) -> Result<(), BevyError> {
        match self.scenes.iter().position(|s| &s.name == scene_id) {
            Some(index) => {
                self.index = index;
                let scene = self.scenes.get_mut(self.index)
                    .context("Non existent scene")?;
                scene.statements_reader.index = 0;
                Ok(())
            },
            None => { Err(BevyError::from(format!("Non existent scene {}", scene_id))) }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub(crate) struct StatementsReader {
    statements: Vec<ast::Statement>,
    index: usize,
}

impl StatementsReader {
    fn new(vec: Vec<ast::Statement>) -> Self {
        Self {
            statements: vec,
            index: 0,
        }
    }
    
    fn advance(&mut self) -> Result<(), BevyError> {
        self.index += 1;
        if self.index + 1 >= self.statements.len() {
            Err(BevyError::from("Scene's statements are finished"))
        } else {
            Ok(())
        }
    }
    
    fn rewind_steps(&self) -> Result<usize, BevyError> {
        if self.index != 0 {
            let search_slice = &self.statements[..self.index];
            let steps = search_slice
                .iter()
                .rposition(|s| { matches!(s, Statement::Dialogue(_)) })
                .map(|pos| self.index - pos)
                .context("Cannot rewind: scene is at its first statement")?;
            Ok(steps)
        } else {
            Err(BevyError::from("Cannot rewind: scene is at its first statement"))
        }
    }
    
    fn rewind(&mut self) {
        self.index = self.index.saturating_sub(1);
    }
    
    fn current(&self) -> Option<ast::Statement> {
        self.statements.get(self.index).cloned()
    }
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
            .init_asset::<Act>()
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