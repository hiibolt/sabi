mod background;
mod character;
mod chat;
mod compiler;
mod loader;

use crate::background::*;
use crate::character::*;
use crate::chat::*;
use crate::compiler::ast::Evaluate;
use crate::compiler::ast::Statement;
use crate::compiler::ast::TextItem;
use crate::compiler::*;
use crate::loader::CharacterJsonLoader;
use crate::loader::PestLoader;

use bevy::prelude::*;
use bevy::ecs::error::ErrorContext;

pub(crate) trait VariantKind {
    fn kind(&self) -> usize;
}

impl VariantKind for ast::Statement {
    fn kind(&self) -> usize {
        match self {
            Statement::TextItem(_) => 1,
            Statement::Stage(_)    => 2,
            Statement::Code(_)     => 3,
        }
    }
}

#[derive(Default)]
pub(crate) struct Cursor<T> {
    data: Vec<T>,
    pos: i32,
}

impl Default for Cursor<ast::Statement> {
    fn default() -> Self {
        Cursor { data: Vec::default(), pos: -1 }
    }
}

impl<T> Cursor<T> {
    pub(crate) fn new(vec: Vec<T>) -> Self {
        Self {
            data: vec,
            pos: -1,
        }
    }
    
    pub(crate) fn next(&mut self) -> Option<T>
    where
        T: Clone
    {
        self.pos += 1;
        self.data.get(self.pos as usize).cloned()
    }

    pub(crate) fn prev(&mut self) -> Option<T>
    where
        T: Clone
    {
        if self.pos == 0 { return None; }
        self.pos -= 1;
        self.data.get(self.pos as usize).cloned()
    }

    pub(crate) fn find_previous(&self) -> Option<T>
    where
        T: Clone + VariantKind
    {
        if let Some(item) = self.data.get(self.pos as usize) {
            let current_kind = item.kind();
            let mut idx: i32 = self.pos as i32 - 1;
            while idx >= 0 {
                if let Some(back_item) = self.data.get(idx as usize) {
                    if back_item.kind() == current_kind {
                        return Some(back_item.clone());
                    }
                } else { return None; }
                idx -= 1;
            }
            None
        } else {
            None
        }
    }
}

/// Resource containing main [Act] state and related runtime data for the Visual Novel.
/// Player-designated constants are passe by the [UserDefinedConstants] resource.
#[derive(Resource, Default)]
pub(crate) struct VisualNovelState {
    // Player-designated constants
    playername: String,

    pub act: Box<ast::Act>,
    pub scene: Box<ast::Scene>,
    pub statements: Cursor<ast::Statement>,
    blocking: bool,
    pub rewinding: usize,
    pub history: Vec<HistoryItem>,
}

pub(crate) enum HistoryItem {
    Statement(ast::Statement),
    Descriptor(String),
}

impl VisualNovelState {
    pub fn set_rewind(&mut self) {
        let search_slice = &self.history[..self.history.len() - 1];
        let last_d = search_slice.iter().rposition(|s| {
            if let HistoryItem::Statement(stm) = s {
                matches!(stm, Statement::TextItem(_))
            } else {
                false
            }
        });
        if let Some(index) = last_d {
            self.rewinding = self.history.len() - (index + 1);
            self.blocking = false;
        }
    }

    pub fn history_summary(&self) -> Result<Vec<String>> {
        let mut text: Vec<String> = Vec::new();

        for statement in &self.history {
            match statement {
                HistoryItem::Statement(s) => {
                    if let Statement::TextItem(t) = s {
                        match t {
                            TextItem::Dialogue(d) => {
                                text.push(d.character.clone() + format!(": {}\n", d.dialogue.evaluate_into_string()?).as_str());
                            },
                            TextItem::InfoText(i) => {
                                text.push(i.infotext.evaluate_into_string()?);
                            }
                        }
                    }
                }
                HistoryItem::Descriptor(s) => {
                    text.push(s.clone() + "\n");
                }
            }
        }

        Ok(text)
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
#[derive(Message)]
pub struct SabiEnd;

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
