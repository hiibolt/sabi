use crate::actor::controller::ActorType;
use crate::chat::controller::InfoTextMessage;
use crate::{BackgroundChangeMessage, CharacterSayMessage, GUIChangeMessage, ActorChangeMessage, VisualNovelState};
use crate::compiler::ast::{CodeStatement, Dialogue, Evaluate, InfoText, StageCommand, Statement, TextItem};
use bevy::prelude::*;
use anyhow::{Context, Result};

/* Messages */
#[derive(Message)]
pub struct SceneChangeMessage {
    pub scene_id: String
}

#[derive(Message)]
pub struct ActChangeMessage {
    pub act_id: String
}

pub struct InvokeContext<'l, 'a, 'b, 'd, 'e, 'f, 'g, 'h, 'i> {
    pub game_state: &'l mut ResMut<'a, VisualNovelState>,
    pub character_say_message: &'l mut MessageWriter<'b, CharacterSayMessage>,
    pub background_change_message: &'l mut MessageWriter<'d, BackgroundChangeMessage>,
    pub gui_change_message: &'l mut MessageWriter<'e, GUIChangeMessage>,
    pub scene_change_message: &'l mut MessageWriter<'f, SceneChangeMessage>,
    pub act_change_message: &'l mut MessageWriter<'g, ActChangeMessage>,
    pub actor_change_message: &'l mut MessageWriter<'h, ActorChangeMessage>,
    pub info_text_message: &'l mut MessageWriter<'i, InfoTextMessage>,
}
pub trait Invoke {
    fn invoke ( &self, ctx: InvokeContext ) -> Result<()>;
}
impl Invoke for Dialogue {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        let dialogue = self.dialogue.evaluate_into_string()
            .context("...while evaluating Dialogue expression")?;
        info!("Invoking Dialogue::Say");

        ctx.character_say_message.write(CharacterSayMessage {
            name: self.character.to_owned(),
            message: dialogue
        });

        ctx.game_state.blocking = true;

        Ok(())
    }
}
impl Invoke for InfoText {
    fn invoke ( &self, ctx: InvokeContext ) -> Result<()> {
        let text = self.infotext.evaluate_into_string()
            .context("...while evaluating InfoText expression")?;
        info!("Invoking InfoText");
        
        ctx.info_text_message.write(InfoTextMessage {
            text,
        });
        
        ctx.game_state.blocking = true;
        
        Ok(())
    }
}
impl Invoke for StageCommand {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        match self {
            StageCommand::BackgroundChange { operation } => {
                info!("Invoking StageCommand::BackgroundChange to {:?}", operation);
                ctx.background_change_message.write(BackgroundChangeMessage {
                    operation: operation.clone(),
                });
            },
            StageCommand::GUIChange { gui_target, sprite_expr, image_mode } => {
                let gui_target = gui_target.clone();
                let sprite_id = sprite_expr.evaluate_into_string()
                    .context("...while evaluating GUIChange sprite expression")?;
                let image_mode = image_mode.clone();
                
                info!("Invoking StageCommand::GUIChange to {:?}'s {}", gui_target, sprite_id);
                ctx.gui_change_message.write(GUIChangeMessage {
                    gui_target,
                    sprite_id,
                    image_mode,
                });
            },
            StageCommand::SceneChange { scene_expr } => {
                let scene_id = scene_expr.evaluate_into_string()
                    .context("...while evaluating SceneChange expression")?;
                
                info!("Invoking StageCommand::SceneChange to {}", scene_id);
                ctx.scene_change_message.write(SceneChangeMessage {
                    scene_id
                });
            },
            StageCommand::ActChange { act_expr } => {
                let act_id = act_expr.evaluate_into_string()
                    .context("...while evaluating ActChange expression")?;
                
                info!("Invoking StageCommand::ActChange to {}", act_id);
                ctx.act_change_message.write(ActChangeMessage {
                    act_id
                });
            },
            StageCommand::CharacterChange { character, operation } => {
                info!("Invoking StageCommand::CharacterChange to {} of type {:?}", character, operation);
                let message = ActorChangeMessage {
                    r#type: ActorType::Character,
                    name: character.clone(),
                    operation: operation.clone()
                };
                if message.is_blocking() {
                    ctx.game_state.blocking = true;
                }
                ctx.actor_change_message.write(message);
            },
            StageCommand::AnimationChange { animation, operation } => {
                info!("Invoking StageCommand::AnimationChange to {} of type {:?}", animation, operation);
                let message = ActorChangeMessage {
                    r#type: ActorType::Animation,
                    name: animation.clone(),
                    operation: operation.clone()
                };
                ctx.actor_change_message.write(message);
            }
        }
        
        Ok(())
    }
}
impl Invoke for CodeStatement {
    fn invoke( &self, _ctx: InvokeContext ) -> Result<()> {
        match self {
            CodeStatement::Log { exprs } => {
                let mut log_parts: Vec<String> = Vec::new();

                for expr in exprs {
                    let part = expr.evaluate_into_string()
                        .context("...while evaluating Log expression")?;
                    log_parts.push(part);
                }

                let log_message = log_parts.join(" ");
                println!("[ Log ] {}", log_message);

                Ok(())
            },
        }
    }
}
impl Invoke for Statement {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        Ok(match self {
            Statement::TextItem(textitem) => {
                match textitem {
                    TextItem::Dialogue(dialogue) => dialogue.invoke(ctx)
                        .context("...while invoking Dialogue statement")?,
                    TextItem::InfoText(infotext) => infotext.invoke(ctx)
                        .context("...while invoking InfoText statement")?, 
                }
            }
            Statement::Stage(stage) => stage.invoke(ctx)
                .context("...while invoking StageCommand statement")?,
            Statement::Code(code) => code.invoke(ctx)
                .context("...while invoking Code statement")?,
        })
    }
}