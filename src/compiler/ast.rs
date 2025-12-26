use pest::{iterators::Pair, pratt_parser::PrattParser};
use pest_derive::Parser;
use anyhow::{bail, ensure, Context, Result};
use bevy::prelude::*;
use std::collections::HashMap;

use crate::{
    actor::{ActorOperation, controller::{ActorPosition, AnimationPosition, CharacterDirection, CharacterPosition, SpawnInfo}}, background::controller::{BackgroundDirection, BackgroundOperation}, chat::controller::{GuiChangeTarget, GuiImageMode}
};

#[derive(Parser)]
#[grammar = "../sabi.pest"]
pub(crate) struct SabiParser;

lazy_static::lazy_static! {
    pub(crate) static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        // Precedence is defined from lowest to highest priority
        PrattParser::new()
            .op(Op::infix(Rule::add, Left))
    };
}

// Trait for evaluating expressions by flattening them
pub(crate) trait Evaluate {
    fn evaluate_into_string(&self) -> Result<String>;
    fn evaluate(&self) -> Result<Expr>;
}

#[derive(Debug, Clone)]
pub(crate) enum Expr {
    Number(f64),
    String(String),
    Add { lhs: Box<Expr>, rhs: Box<Expr> }
}

impl Evaluate for Expr {
    fn evaluate_into_string(&self) -> Result<String> {
        let evaluated = self.evaluate()
            .context("Failed to evaluate expression")?;
        expr_to_string(&evaluated)
            .context("Failed to convert evaluated expression to string")
    }
    fn evaluate(&self) -> Result<Expr> {
        match self {
            Expr::String(_) | Expr::Number(_) => Ok(self.clone()),
            Expr::Add { lhs, rhs } => {
                let left = lhs.evaluate().context("Failed to evaluate left side of addition")?;
                let right = rhs.evaluate().context("Failed to evaluate right side of addition")?;

                match (&left, &right) {
                    (Expr::Number(l), Expr::Number(r)) => {
                        Ok(Expr::Number(l + r))
                    },
                    (Expr::String(l), Expr::String(r)) => {
                        Ok(Expr::String(format!("{}{}", l, r)))
                    },
                    (Expr::Number(n), Expr::String(s)) => {
                        Ok(Expr::String(format!("{}{}", n, s)))
                    },
                    (Expr::String(s), Expr::Number(n)) => {
                        Ok(Expr::String(format!("{}{}", s, n)))
                    },
                    _ => {
                        // For complex expressions, convert to strings and concatenate
                        let left_str = expr_to_string(&left)?;
                        let right_str = expr_to_string(&right)?;
                        Ok(Expr::String(format!("{}{}", left_str, right_str)))
                    }
                }
            }
        }
    }
}

// Helper function to convert Expr to String
pub(crate) fn expr_to_string(expr: &Expr) -> Result<String> {
    match expr {
        Expr::String(s) => Ok(s.clone()),
        Expr::Number(n) => Ok(n.to_string()),
        Expr::Add { .. } => {
            let evaluated = expr.evaluate()?;
            expr_to_string(&evaluated)
        }
    }
}

#[derive(Debug, Clone, Default, Asset, TypePath)]
pub(crate) struct Act {
    pub scenes: HashMap<String, Box<Scene>>,
    pub name: String,
    pub entrypoint: String,
}

#[derive(Debug, Clone)]
pub(crate) enum CodeStatement {
    Log { exprs: Vec<Expr> }
}

#[derive(Debug, Clone)]
pub(crate) enum StageCommand {
    BackgroundChange { operation: BackgroundOperation },
    GUIChange { gui_target: GuiChangeTarget, sprite_expr: Box<Expr>, image_mode: GuiImageMode },
    SceneChange { scene_expr: Box<Expr> },
    ActChange { act_expr: Box<Expr> },
    CharacterChange { character: String, operation: ActorOperation },
    AnimationChange { animation: String, operation: ActorOperation },
}

#[derive(Debug, Clone)]
pub(crate) enum TextItem {
    Dialogue(Dialogue),
    InfoText(InfoText),
}

#[derive(Debug, Clone)]
pub(crate) struct InfoText {
    pub infotext: Expr
}

#[derive(Debug, Clone)]
pub(crate) struct Dialogue {
    pub character: String,
    pub dialogue: Expr
}

#[derive(Debug, Clone)]
pub(crate) enum Statement {
    Code(CodeStatement),
    Stage(StageCommand),
    TextItem(TextItem)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Scene {
    pub name: String,
    pub statements: Vec<Statement>,
}

impl PartialEq for Scene {
    fn eq(&self, other: &Self) -> bool {
        other.name == self.name
    }
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

pub(crate) fn build_expression(pair: pest::iterators::Pair<Rule>) -> Result<Expr> {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::number => {
                primary.as_str().parse::<f64>()
                    .map(Expr::Number)
                    .context("Failed to parse number")
            }
            Rule::string => {
                let s = primary.as_str();
                // Remove the surrounding quotes
                let s = &s[1..s.len()-1];
                Ok(Expr::String(s.to_string()))
            },
            Rule::expr => build_expression(primary),
            other => bail!("Unexpected primary expr: {other:?}"),
        })
        .map_infix(|left, op, right| {
            match op.as_rule() {
                Rule::add => Ok(Expr::Add {
                    lhs: Box::new(left.context("Failed to evaluate left operand")?),
                    rhs: Box::new(right.context("Failed to evaluate right operand")?),
                }),
                other => bail!("Unexpected infix operator: {other:?}"),
            }
        })
        .parse(pair.into_inner())
        .context("Failed to parse expression")
}

fn build_actor_spawn_directive(character: &str, action: &str, mut action_iter: pest::iterators::Pairs<'_, Rule>) -> Result<StageCommand> {
    let operation = match action {
        "appears" | "fade in" => {
            let mut info = SpawnInfo {
                fading: action == "fade in",
                ..Default::default()
            };
            while let Some(a) = action_iter.next() {
                match a.as_rule() {
                    Rule::emotion_name => { info.emotion = Some(a.as_str().to_owned()); },
                    Rule::character_position => {
                        let position = match CharacterPosition::try_from(a.as_str()) {
                            Ok(pos) => pos,
                            Err(e) => bail!(e)
                        };
                        info.position = Some(ActorPosition::Character(position));
                    }
                    Rule::actor_direction_directive => {
                        let mut inner_rules = a.into_inner();
                        let _ = inner_rules.next().context("Could not get action direction command")?;
                        let direction = inner_rules.next().context("Could not get actor direction")?;
                        match direction.as_str() {
                            "left" => info.direction = CharacterDirection::Left,
                            "right" => info.direction = CharacterDirection::Right,
                            other => { return Err(anyhow::anyhow!("Unhandled direction provided {:?}", other).into()); }
                        };
                    },
                    other => { bail!("Unexpected added action to character spawn directive! {:?}", other); }
                };
            }
            ActorOperation::Spawn(info)
        },
        "disappears" | "fade out" => ActorOperation::Despawn(action == "fade out"),
        other => bail!("Unexpected actor spawn operation: {:?}", other)
    };
    
    Ok(StageCommand::CharacterChange { character: character.to_string(), operation })
}

fn build_character_direction_directive(character: &str, action: Pair<'_, Rule>) -> Result<StageCommand> {
    let mut direction_iter = action.into_inner();
    let _ = direction_iter.next().context("Could not get direction command")?;
    let direction = direction_iter.next().context("Could not get direction")?;
    let direction = match direction.as_rule() {
        Rule::actor_direction => {
            match direction.as_str() {
                "left" => CharacterDirection::Left,
                "right" => CharacterDirection::Right,
                other => { bail!("Unhandled direction provided {:?}", other); }
            }
        },
        other => { bail!("Character direction directive needs direction argument [\"left\", \"right\"], found {:?}", other); }
    };
    Ok(StageCommand::CharacterChange { character: character.to_string(), operation: ActorOperation::Look(direction) })
}

fn build_actor_movement_directive(actor: &str, action: &str, mut action_iter: pest::iterators::Pairs<'_, Rule>) -> Result<StageCommand> {
    match action {
        "moves" => {
            match action_iter.peek() {
                Some(n) if n.as_rule() == Rule::character_position => {
                    let position_pair = action_iter.next().context("Expected actor position")?;
                    ensure!(position_pair.as_rule() == Rule::character_position,
                        "Expected character position, found {:?}", position_pair.as_rule());

                    match CharacterPosition::try_from(position_pair.as_str()) {
                        Ok(pos) => { return Ok(StageCommand::CharacterChange { character: actor.to_string(), operation: ActorOperation::Move(ActorPosition::Character(pos)) }) },
                        Err(e) => bail!(e)
                    }
                },
                Some(n) if n.as_rule() == Rule::animation_position => {
                    let position_pair = action_iter.next().context("Expected actor position")?;
                    ensure!(position_pair.as_rule() == Rule::animation_position,
                        "Expected animation position, found {:?}", position_pair.as_rule());

                    match AnimationPosition::try_from(position_pair.as_str()) {
                        Ok(pos) => { return Ok(StageCommand::AnimationChange { animation: actor.to_string(), operation: ActorOperation::Move(ActorPosition::Animation(pos)) }) },
                        Err(e) => bail!(e)
                    }
                }
                _ => { bail!("Animation move directive needs position arguments"); }
            }
        }
        other => bail!("Unexpected action in Actor Direction Directive command: {:?}", other)
    }
}

pub(crate) fn build_stage_command(pair: Pair<Rule>) -> Result<Statement> {
    ensure!(pair.as_rule() == Rule::stage_command,
        "Expected stage rule, found {:?}", pair.as_rule());

    let command_pair = pair.into_inner().next()
        .context("Stage command missing inner command")?;

    let result = match command_pair.as_rule() {
        Rule::background_change => {
            let mut inner = command_pair.into_inner();
            let background_operation = inner.next().context("Background operation missing")?;
            let action = background_operation.into_inner().next().context("Background action missing")?;

            ensure!(action.as_rule() == Rule::background_action,
                "Expected background action, found {:?}", action.as_rule());

            let def = action.into_inner().next().context("Invalid background action")?;

            let operation = match def.as_rule() {
                Rule::background_change_def => {
                    let target = def.into_inner().next()
                        .context("Background - Missing change operation target")?
                        .as_str().trim_matches('"').to_owned();
                    BackgroundOperation::ChangeTo(target)
                },
                Rule::background_dissolve_def => {
                    let target = match def.into_inner().next() {
                        Some(rule) => Some(rule.as_str().trim_matches('"').to_owned()),
                        None => None
                    };
                    BackgroundOperation::DissolveTo(target)
                },
                Rule::background_slide_def => {
                    let direction_rule = def.into_inner().next().context("Background direction missing")?;
                    ensure!(direction_rule.as_rule() == Rule::background_direction,
                        "Expected background direction, found {:?}", direction_rule);

                    let direction = match direction_rule.as_str() {
                        "N" | "North" => BackgroundDirection::North,
                        "S" | "South" => BackgroundDirection::South,
                        "E" | "East" => BackgroundDirection::East,
                        "W" | "West" => BackgroundDirection::West,
                        other => bail!("Unidentified direction {}", other)
                    };
                    BackgroundOperation::SlideTo(direction)
                },
                _ => { bail!("Invalid background action"); }
            };

            StageCommand::BackgroundChange { operation }
        },
        Rule::gui_change => {
            let mut inner = command_pair.into_inner();
            let gui_element_pair = inner.next()
                .context("GUI change missing GUI element")?;
            let sprite_expr_pair = inner.next()
                .context("GUI change missing sprite expression")?;

            // Convert gui_element to the appropriate ID
            let gui_target = match gui_element_pair.as_str() {
                "textbox" => GuiChangeTarget::TextBoxBackground,
                "namebox" => GuiChangeTarget::NameBoxBackground,
                other => bail!("Unknown GUI element: {}", other)
            };

            let sprite_expr = build_expression(sprite_expr_pair)
                .context("Failed to build sprite expression for GUI change")?;

            let image_mode = if let Some(image_mode) = inner.next() {
                ensure!(image_mode.as_rule() == Rule::image_mode,
                    "Expected image mode, found {:?}", image_mode.as_rule());
                match image_mode.as_str() {
                    "sliced" => GuiImageMode::Sliced,
                    other => bail!("Unrecognized image mode definition: {}", other)
                }
            } else { GuiImageMode::Auto };

            StageCommand::GUIChange {
                gui_target,
                sprite_expr: Box::new(sprite_expr),
                image_mode,
            }
        },
        Rule::scene_change => {
            let expr_pair = command_pair.into_inner().next()
                .context("Scene change missing expression")?;
            let expr = build_expression(expr_pair)
                .context("Failed to build expression for scene change")?;
            StageCommand::SceneChange { scene_expr: Box::new(expr) }
        },
        Rule::act_change => {
            let expr_pair = command_pair.into_inner().next()
                .context("Act change missing expression")?;
            let expr = build_expression(expr_pair)
                .context("Failed to build expression for act change")?;
            StageCommand::ActChange { act_expr: Box::new(expr) }
        },
        Rule::character_change => {
            let mut inner_rules = command_pair.into_inner();
            let character = inner_rules.next()
                .context("Character change missing character identifier")?
                .as_str()
                .to_owned();
            let action = inner_rules.next()
                .context("Character change missing character action")?;
            let mut action_iter = action.into_inner();
            let action = action_iter.next().context("Could not get inner action")?;
            match action.as_rule() {
                Rule::actor_spawn_directive         => { build_actor_spawn_directive(&character, action.as_str(), action_iter)? }
                Rule::actor_direction_directive     => { build_character_direction_directive(&character, action)? },
                Rule::actor_movement_directive  => { build_actor_movement_directive(&character, action.as_str(), action_iter)? },
                other => { bail!("Unexpected rule in character_action {:?}", other); }
            }
        },
        Rule::animation_change => {
            let mut inner_rules = command_pair.into_inner();
            let animation_identifier = inner_rules.next().context("Could not get animation identifier")?;
            let mut anim_id_rules = animation_identifier.into_inner();
            let animation = anim_id_rules.next()
                .context("Animation change missing animation identifier")?
                .as_str()
                .trim()
                .trim_matches('"')
                .to_owned();
            let animation_action = inner_rules.next()
                .context("Animation change missing spawn_directive")?;
            ensure!(animation_action.as_rule() == Rule::animation_action,
                "Expected animation action, found {:?}", animation_action.as_rule());
            
            let mut inner_rules = animation_action.into_inner();
            let directive = inner_rules.next().context("Could not get animation action elements")?;
            
            match directive.as_rule() {
                Rule::actor_spawn_directive => {
                    match directive.as_str() {
                        "appears" | "fade in" => {
                            let mut spawn_info = SpawnInfo::default();
                            
                            while let Some(directive) = inner_rules.next() {
                                match directive.as_rule() {
                                    Rule::animation_position => {
                                        let position = AnimationPosition::try_from(directive.as_str())?;
                                        spawn_info.position = Some(ActorPosition::Animation(position));
                                    },
                                    Rule::actor_direction_directive => {
                                        let mut pair_iter = directive.into_inner();
                                        let _ = pair_iter.next().context("Missing direction command")?;
                                        let direction = pair_iter.next().context("Missing direction")?;
                                        spawn_info.direction = CharacterDirection::try_from(direction.as_str())?;
                                    },
                                    Rule::animation_scale => {
                                        let mut pair_iter = directive.into_inner();
                                        let scale = pair_iter.next().context("Missing scale value")?;
                                        let number: f32 = if scale.as_str().contains(".") { scale.as_str().parse()? } else { scale.as_str().parse::<i32>()? as f32 };
                                        spawn_info.scale = Some(number);
                                    },
                                    other => info!("Unexpected rule in animation definition! {:?}", other)
                                }
                            }
                            
                            StageCommand::AnimationChange { animation, operation: ActorOperation::Spawn(spawn_info) }
                        },
                        "disappears" |  "fade out" => {
                            StageCommand::AnimationChange { animation, operation: ActorOperation::Despawn(directive.as_str() == "fade out") }
                        },
                        other => { return Err(anyhow::anyhow!("Unexpected spawn directive! {}", other).into()); }
                    }
                },
                Rule::actor_movement_directive => {
                    build_actor_movement_directive(&animation, directive.as_str(), inner_rules)?
                },
                other => { return Err(anyhow::anyhow!("Unexpected directive! {:?}", other).into()); }
            }
        }
        other => bail!("Unexpected rule in stage command: {:?}", other)
    };

    Ok(Statement::Stage(result))
}

pub fn build_code_statement(code_pair: Pair<Rule>) -> Result<Statement> {
    ensure!(code_pair.as_rule() == Rule::code,
        "Expected code rule, found {:?}", code_pair.as_rule());

    let statement_pair = code_pair.into_inner().next()
        .context("Code block missing statement")?;

    let result = match statement_pair.as_rule() {
        Rule::log => {
            let mut exprs = Vec::new();
            for expr_pair in statement_pair.into_inner() {
                let expr = build_expression(expr_pair)
                    .context("Failed to build expression for log statement")?;
                exprs.push(expr);
            }
            CodeStatement::Log { exprs }
        },
        other => bail!("Unexpected rule in code statement: {:?}", other)
    };

    Ok(Statement::Code(result))
}

pub fn build_dialogue(pair: Pair<Rule>) -> Result<Vec<Statement>> {
    ensure!(pair.as_rule() == Rule::dialogue,
        "Expected dialogue, found {:?}", pair.as_rule());

    let mut inner_rules = pair.into_inner().peekable();

    let character = inner_rules.next()
        .context("Dialogue missing character identifier")?
        .as_str()
        .to_owned();

    let emotion_statement = match inner_rules.peek() {
        Some(n) if n.as_rule() == Rule::dialogue_emotion_change => {
            let emotion_pair = inner_rules.next()
                .context("Expected emotion pair")?;
            let emotion_name_pair = emotion_pair.into_inner().next()
                .context("Emotion change missing emotion name")?;

            ensure!(emotion_name_pair.as_rule() == Rule::emotion_name,
                "Expected emotion name, found {:?}", emotion_name_pair.as_rule());

            Some(Statement::Stage(StageCommand::CharacterChange {
                character: character.clone(),
                operation: ActorOperation::EmotionChange(emotion_name_pair.as_str().to_owned())
            }))
        },
        _ => None
    };

    let initial_dialogue_statement = {
        let dialogue_text_pair = inner_rules.next()
            .context("Dialogue missing dialogue text")?;
        ensure!(dialogue_text_pair.as_rule() == Rule::expr,
            "Expected dialogue text, found {:?}", dialogue_text_pair.as_rule());

        let dialogue = build_expression(dialogue_text_pair)
            .context("Failed to build expression for dialogue text")?;

        Statement::TextItem(TextItem::Dialogue(Dialogue  {
            character: character.clone(),
            dialogue
        }))
    };

    let statements = {
        let mut statements = vec!(initial_dialogue_statement);
        if let Some(emotion_stmt) = emotion_statement {
            statements.insert(0, emotion_stmt);
        }

        while let Some(dialogue_text_pair) = inner_rules.next() {
            match dialogue_text_pair.as_rule() {
                Rule::expr => {
                    let dialogue = build_expression(dialogue_text_pair)
                        .context("Failed to build expression for dialogue text")?;

                    statements.push(Statement::TextItem(TextItem::Dialogue(Dialogue {
                        character: character.clone(),
                        dialogue
                    })));
                },
                Rule::stage_command => {
                    let stage_stmt = build_stage_command(dialogue_text_pair)?;
                    statements.push(stage_stmt);
                },
                other => bail!("Unexpected rule in dialogue text: {:?}", other)
            }
        }

        statements
    };

    Ok(statements)
}

pub fn build_infotext(pair: Pair<Rule>) -> Result<Statement> {
    let mut pairs = pair.into_inner();
    let narrator_pair = pairs.next()
        .context("Infotext rule missing inner elements")?;
    
    ensure!(narrator_pair.as_rule() == Rule::narrator,
        "InfoText has no 'narrator' as speaker: {:?}", narrator_pair.as_rule());
    
    let infotext = pairs.next()
        .context("Infotext missing text")?;
    
    ensure!(infotext.as_rule() == Rule::expr,
        "Expected dialogue text, found {:?}", infotext.as_rule());
    
    let infotext = build_expression(infotext)
        .context("Failed to build expression for infotext")?;
    
    Ok(Statement::TextItem(TextItem::InfoText(InfoText { infotext })))
}

pub fn build_scenes(pair: Pair<Rule>) -> Result<Act> {
    let mut act = Act::default();

    let mut first_scene_id: Option<String> = None;

    for scene_pair in pair.into_inner() {
        match scene_pair.as_rule() {
            Rule::scene => {
                let mut inner_rules = scene_pair.into_inner();

                let scene_id = inner_rules.next()
                    .context("Scene missing ID")?
                    .as_str()
                    .to_owned();

                // Set the first scene as entrypoint
                if first_scene_id.is_none() {
                    first_scene_id = Some(scene_id.clone());
                }

                let mut statements = Vec::new();
                for statement_pair in inner_rules {
                    let stmt = match statement_pair.as_rule() {
                        Rule::code => build_code_statement(statement_pair)?,
                        Rule::stage_command => build_stage_command(statement_pair)?,
                        Rule::text_item => {
                            let text_item = statement_pair.into_inner().next()
                                .context("No text item rule found")?;
                            match text_item.as_rule() {
                                Rule::infotext => build_infotext(text_item)?,
                                Rule::dialogue => {
                                    let mut inner_statements = build_dialogue(text_item.clone())?;
                                    statements.extend(inner_statements.drain(..));
        
                                    continue;
                                },
                                other => bail!("Invalid text item rule in scene: {:?}", other)
                            }
                        }
                        other => bail!("Unexpected rule in scene: {:?}", other),
                    };
                    statements.push(stmt);
                }

                ensure!(act.scenes.insert(scene_id.clone(), Box::new(Scene { name: scene_id.clone(), statements })).is_none(), "Duplicate scene ID '{}'", scene_id);
            },
            Rule::EOI => continue,
            other => bail!("Unexpected rule when parsing scenes: {:?}", other),
        }
    }

    act.entrypoint = first_scene_id.context("No scenes found in act")?;
    Ok(act)
}
