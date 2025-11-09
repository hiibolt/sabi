use std::{collections::HashMap, iter::Peekable};
use pest::{iterators::Pair, pratt_parser::PrattParser};
use pest_derive::Parser;
use anyhow::{bail, ensure, Context, Result};

use crate::{character::{CharacterOperation, controller::{CharacterDirection, CharacterPosition, SpawnInfo}}, chat::controller::GuiChangeTarget};

#[derive(Parser)]
#[grammar = "../sabi.pest"]
pub struct SabiParser;

lazy_static::lazy_static! {
    pub static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        // Precedence is defined from lowest to highest priority
        PrattParser::new()
            .op(Op::infix(Rule::add, Left))
    };
}

// Trait for evaluating expressions by flattening them
pub trait Evaluate {
    fn evaluate_into_string(&self) -> Result<String>;
    fn evaluate(&self) -> Result<Expr>;
}

#[derive(Debug, Clone)]
pub enum Expr {
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
pub fn expr_to_string(expr: &Expr) -> Result<String> {
    match expr {
        Expr::String(s) => Ok(s.clone()),
        Expr::Number(n) => Ok(n.to_string()),
        Expr::Add { .. } => {
            let evaluated = expr.evaluate()?;
            expr_to_string(&evaluated)
        }
    }
}


#[derive(Debug, Clone, Default)]
pub struct Act {
    pub scenes: HashMap<String, Box<Scene>>,
    pub entrypoint: String,
}
pub type Acts = HashMap<String, Box<Act>>;

#[derive(Debug, Clone)]
pub enum CodeStatement {
    Log { exprs: Vec<Expr> }
}

#[derive(Debug, Clone)]
pub enum StageCommand {
    BackgroundChange { background_expr: Box<Expr> },
    GUIChange { gui_target: GuiChangeTarget, sprite_expr: Box<Expr> },
    SceneChange { scene_expr: Box<Expr> },
    ActChange { act_expr: Box<Expr> },
    CharacterChange { character: String, operation: CharacterOperation },
}

#[derive(Debug, Clone)]
pub struct Dialogue {
    pub character: String,
    pub dialogue: Expr
}

#[derive(Debug, Clone)]
pub enum Statement {
    Code(CodeStatement),
    Stage(StageCommand),
    Dialogue(Dialogue)
}


#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub statements: Vec<Statement>
}

pub fn build_expression(pair: pest::iterators::Pair<Rule>) -> Result<Expr> {
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

fn build_character_spawn_directive(character: &str, action: &str, mut inner_action: Peekable<pest::iterators::Pairs<'_, Rule>>) -> Result<StageCommand> {
    let result = match action {
        "appears" | "fade in" => {
            let mut info = SpawnInfo {
                fading: action == "fade in",
                ..Default::default()
            };
            let mut added_action = Some(());
            while let Some(_) = added_action {
                match inner_action.peek() {
                    Some(n) if n.as_rule() == Rule::emotion_name => {
                        let emotion_pair = inner_action.next()
                            .context("Expected emotion pair")?;
                     
                        ensure!(emotion_pair.as_rule() == Rule::emotion_name,
                            "Expected emotion name, found {:?}", emotion_pair.as_rule());
                        info.emotion = Some(emotion_pair.as_str().to_owned());
                    },
                    Some(n) if n.as_rule() == Rule::character_position => {
                        let position_pair = inner_action.next()
                            .context("Expected position pair")?;
                        
                        ensure!(position_pair.as_rule() == Rule::character_position,
                            "Expecter position, found {:?}", position_pair.as_rule());
                        let position = match CharacterPosition::try_from(position_pair.as_str()) {
                            Ok(pos) => pos,
                            Err(e) => bail!(e)
                        };
                        info.position = position;
                    }
                    _ => {added_action = None;}
                };
            }
            StageCommand::CharacterChange { character: character.to_string(), operation: CharacterOperation::Spawn(info) }
        },
        "disappears" | "fade out" => {
            StageCommand::CharacterChange { character: character.to_string(), operation: CharacterOperation::Despawn(action == "fade out") }
        },
        other => bail!("Unexpected action in Character Spawn Directive command: {:?}", other)
    };
    Ok(result)
}

fn build_character_direction_directive(character: &str, action: &str, mut inner_action: Peekable<pest::iterators::Pairs<'_, Rule>>) -> Result<StageCommand> {
    let result = match action {
        "looking" | "looks" => {
            let direction = match inner_action.peek() {
                Some(n) if n.as_rule() == Rule::character_direction => {
                    let direction_pair = inner_action.next().context("Expected character direction")?;
                    ensure!(direction_pair.as_rule() == Rule::character_direction,
                        "Expected character direction, found {:?}", direction_pair.as_rule());
                    
                    match direction_pair.as_str() {
                        "left" => CharacterDirection::Left,
                        "right" => CharacterDirection::Right,
                        other => { bail!("Unhandled direction provided {:?}", other); }
                    }
                    
                },
                _ => { bail!("Character direction directive needs direction argument [\"left\", \"right\"]"); }
            };
            StageCommand::CharacterChange { character: character.to_string(), operation: CharacterOperation::Look(direction) }
        }
        other => bail!("Unexpected action in Character Direction Directive command: {:?}", other)
    };
    Ok(result)
}

fn build_character_movement_directive(character: &str, action: &str, mut inner_action: Peekable<pest::iterators::Pairs<'_, Rule>>) -> Result<StageCommand> {
    let result = match action {
        "moves" => {
            let position = match inner_action.peek() {
                Some(n) if n.as_rule() == Rule::character_position => {
                    let position_pair = inner_action.next().context("Expected character direction")?;
                    ensure!(position_pair.as_rule() == Rule::character_position,
                        "Expected character position, found {:?}", position_pair.as_rule());
                    
                    match CharacterPosition::try_from(position_pair.as_str()) {
                        Ok(pos) => pos,
                        Err(e) => bail!(e)
                    }
                },
                _ => { bail!("Character move directive needs position argument [\"center\", \"left\", \"right\", \"invisible left\", \"invisible right\"]"); }
            };
            StageCommand::CharacterChange { character: character.to_string(), operation: CharacterOperation::Move(position) }
        }
        other => bail!("Unexpected action in Character Direction Directive command: {:?}", other)
    };
    Ok(result)
}

pub fn build_stage_command(pair: Pair<Rule>) -> Result<Statement> {
    ensure!(pair.as_rule() == Rule::stage_command, 
        "Expected stage rule, found {:?}", pair.as_rule());
    
    let command_pair = pair.into_inner().next()
        .context("Stage command missing inner command")?;
    
    let result = match command_pair.as_rule() {
        Rule::background_change => {
            let expr_pair = command_pair.into_inner().next()
                .context("Background change missing expression")?;
            let expr = build_expression(expr_pair)
                .context("Failed to build expression for background change")?;
            StageCommand::BackgroundChange { background_expr: Box::new(expr) }
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
            
            StageCommand::GUIChange { 
                gui_target, 
                sprite_expr: Box::new(sprite_expr) 
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
            let mut inner_rules = command_pair.into_inner().peekable();
            let character = inner_rules.next()
                .context("Character change missing character identifier")?
                .as_str()
                .to_owned();
            let action = inner_rules.next()
                .context("Character change missing character action")?;
            let mut inner_action = action.into_inner().peekable();
            let action = inner_action.next()
                .context("Character action missing action identifier")?;
            match action.as_rule() {
                Rule::character_spawn_directive => { build_character_spawn_directive(&character, action.as_str(), inner_action)? }
                Rule::character_direction_directive => { build_character_direction_directive(&character, action.as_str(), inner_action)? },
                Rule::character_movement_directive => { build_character_movement_directive(&character, action.as_str(), inner_action)? },
                other => { bail!("Unexpected rule in character_action {:?}", other); }
            }
        },
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
                operation: CharacterOperation::EmotionChange(emotion_name_pair.as_str().to_owned())
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
        
        Statement::Dialogue(Dialogue {
            character: character.clone(),
            dialogue
        })
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

                    statements.push(Statement::Dialogue(Dialogue {
                        character: character.clone(),
                        dialogue
                    }));
                },
                Rule::stage_command => {
                    let stage_stmt = build_stage_command(dialogue_text_pair)
                        .context("Failed to build stage command inside dialogue")?;
                    statements.push(stage_stmt);
                },
                other => bail!("Unexpected rule in dialogue text: {:?}", other)
            }
        }

        statements
    };
    
    Ok(statements)
}

pub fn build_scenes(pair: Pair<Rule>) -> Result<Act> {
    let mut act = Act {
        scenes: HashMap::new(),
        entrypoint: String::new(),
    };
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
                        Rule::code => build_code_statement(statement_pair)
                            .context("Failed to build code statement")?,
                        Rule::stage_command => build_stage_command(statement_pair)
                            .context("Failed to build stage command")?,
                        Rule::dialogue => {
                            let mut inner_statements = build_dialogue(statement_pair)
                                .context("Failed to build dialogue")?;
                            statements.extend(inner_statements.drain(..));

                            continue;
                        },
                        other => bail!("Unexpected rule in scene: {:?}", other),
                    };
                    statements.push(stmt);
                }
                
                ensure!(act.scenes.insert(scene_id.clone(), Box::new(Scene { statements })).is_none(), "Duplicate scene ID '{}'", scene_id);
            },
            Rule::EOI => continue,
            other => bail!("Unexpected rule when parsing scenes: {:?}", other),
        }
    }
    
    act.entrypoint = first_scene_id.context("No scenes found in act")?;
    Ok(act)
}