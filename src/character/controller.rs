use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use bevy::{asset::{LoadState, LoadedFolder}, prelude::*};
use serde::Deserialize;

use crate::{VisualNovelState, character::character_operations::{apply_alpha, change_character_emotion, move_characters, spawn_character}, compiler::controller::{Controller, ControllerReadyMessage, SabiState, ControllersSetStateMessage}};
use crate::compiler::controller::UiRoot;

pub const INVISIBLE_LEFT_PERCENTAGE: f32 = -40.;
pub const FAR_LEFT_PERCENTAGE: f32 = 5.;
pub const FAR_RIGHT_PERCENTAGE: f32 = 65.;
pub const LEFT_PERCENTAGE: f32 = 20.;
pub const CENTER_PERCENTAGE: f32 = 35.;
pub const RIGHT_PERCENTAGE: f32 = 50.;
pub const INVISIBLE_RIGHT_PERCENTAGE: f32 = 140.;
const CHARACTERS_ASSET_PATH: &str = "sabi/characters";

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) enum CharacterControllerState {
    #[default]
    Idle,
    Loading,
    Running,
}

impl From<SabiState> for CharacterControllerState {
    fn from(value: SabiState) -> Self {
        match value {
            SabiState::Idle => CharacterControllerState::Idle,
            SabiState::WaitingForControllers => CharacterControllerState::Loading,
            SabiState::Running => CharacterControllerState::Running,
        }
    }
}

/* Components */
#[derive(Component, Debug, Default, Asset, TypePath, Deserialize, Clone)]
pub struct CharacterConfig {
    pub name: String,
    pub outfit: String,
    pub emotion: String,
    pub description: String,
    pub emotions: Vec<String>,
    pub outfits: Vec<String>,
}

#[derive(Component, Default, Debug, Clone, PartialEq)]
pub enum CharacterPosition {
    #[default]
    Center,
    FarLeft,
    FarRight,
    Left,
    Right,
    InvisibleLeft,
    InvisibleRight,
}

impl CharacterPosition {
    pub fn to_percentage_value(&self) -> f32 {
        match &self {
            CharacterPosition::Center => CENTER_PERCENTAGE,
            CharacterPosition::FarLeft => FAR_LEFT_PERCENTAGE,
            CharacterPosition::FarRight => FAR_RIGHT_PERCENTAGE,
            CharacterPosition::Left => LEFT_PERCENTAGE,
            CharacterPosition::Right => RIGHT_PERCENTAGE,
            CharacterPosition::InvisibleLeft => INVISIBLE_LEFT_PERCENTAGE,
            CharacterPosition::InvisibleRight => INVISIBLE_RIGHT_PERCENTAGE
        }
    }
}

impl TryFrom<&str> for CharacterPosition {
    type Error = BevyError;
    
    fn try_from(value: &str) -> Result<Self, BevyError> {
        match value {
            "center" => Ok(CharacterPosition::Center),
            "far left" => Ok(CharacterPosition::FarLeft),
            "far right" => Ok(CharacterPosition::FarRight),
            "left" => Ok(CharacterPosition::Left),
            "right" => Ok(CharacterPosition::Right),
            "invisible left" => Ok(CharacterPosition::InvisibleLeft),
            "invisible right" => Ok(CharacterPosition::InvisibleRight),
            other => { Err(anyhow::anyhow!("Unhandled direction provided {:?}", other).into()) }
        }
    }
}

/* Resources */
#[derive(Resource)]
struct HandleToCharactersFolder(Handle<LoadedFolder>);
#[derive(Resource)]
pub struct CharactersResource(pub CharacterSprites);
#[derive(Resource)]
struct Configs(CharactersConfig);
#[derive(Resource, Default)]
pub struct FadingCharacters(pub Vec<(Entity, f32, bool)>); // entity, alpha_step, to_despawn
#[derive(Resource, Default)]
pub struct MovingCharacters(pub Vec<(Entity, f32)>); // entity, target_position

/* Custom types */
#[derive(Hash, Eq, PartialEq, Debug)]
pub struct SpriteKey {
    pub character: String,
    pub outfit: String,
    pub emotion: String,
}
type CharacterSprites = HashMap<SpriteKey, Handle<Image>>;
type CharactersConfig = HashMap<String, CharacterConfig>;

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterDirection {
    Left,
    Right
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct SpawnInfo {
    pub emotion: Option<String>,
    pub position: CharacterPosition,
    pub fading: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterOperation {
    Spawn(SpawnInfo), 
    EmotionChange(String),
    Despawn(bool), // fading
    Look(CharacterDirection),
    Move(CharacterPosition),
}

/* Messages */
#[derive(Message)]
pub struct CharacterChangeMessage {
    pub character: String,
    pub operation: CharacterOperation,
}

impl CharacterChangeMessage {
    pub fn is_blocking(&self) -> bool {
        match &self.operation {
            CharacterOperation::Spawn(info) => {
                if info.fading { true } else { false }
            },
            CharacterOperation::Despawn(true) => true,
            _ => false
        }
    }
}

pub struct CharacterController;
impl Plugin for CharacterController {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovingCharacters::default())
            .insert_resource(FadingCharacters::default())
            .add_message::<CharacterChangeMessage>()
            .init_state::<CharacterControllerState>()
            .add_systems(Update, wait_trigger)
            .add_systems(OnEnter(CharacterControllerState::Loading), import_characters)
            .add_systems(Update, setup.run_if(in_state(CharacterControllerState::Loading)))
            .add_systems(Update, (update_characters, apply_alpha, move_characters)
                .run_if(in_state(CharacterControllerState::Running)));
    }
}
fn define_characters_map(
    mut commands: Commands,
    config_res: Res<Assets<CharacterConfig>>,
    loaded_folder: &LoadedFolder,
) -> Result<(), BevyError> {
    let mut characters_sprites = CharacterSprites::new();
    let mut characters_configs = CharactersConfig::new();
    let expected_len = PathBuf::from(CHARACTERS_ASSET_PATH).iter().count() + 3;
    for handle in &loaded_folder.handles {
        let path = handle
            .path()
            .context("Error retrieving character asset path")?
            .path();
        let name: String = match path.iter().nth(expected_len - 3).map(|s| s.to_string_lossy().into()) {
            Some(name) => name,
            None => continue,
        };
        if path.iter().count() == expected_len {
            let outfit = match path.iter().nth(expected_len - 2).map(|s| s.to_string_lossy().into()) {
                Some(outfit) => outfit,
                None => continue,
            };
            let emotion = match path.iter().nth(expected_len - 1) {
                Some(os_str) => {
                    let file = std::path::Path::new(os_str);
                    let name = file.file_stem().map(|s| s.to_string_lossy().into_owned());
                    if let Some(n) = name { n } else { continue }
                }
                None => continue,
            };
            let key = SpriteKey {
                character: name,
                outfit,
                emotion,
            };

            characters_sprites.insert(key, handle.clone().typed());
            
        } else if path.iter().count() == expected_len - 1 {
            characters_configs.insert(
                name.clone(),
                config_res
                    .get(&handle.clone().typed::<CharacterConfig>())
                    .context(format!("Failed to retrieve CharacterConfig for '{}'", name))?
                    .clone(),
            );
        }
    }
    commands.insert_resource(CharactersResource(characters_sprites));
    commands.insert_resource(Configs(characters_configs));
    Ok(())
}
fn setup(
    commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    folder_handle: Res<HandleToCharactersFolder>,
    configs: Res<Assets<CharacterConfig>>,
    mut controller_state: ResMut<NextState<CharacterControllerState>>,
    mut ev_writer: MessageWriter<ControllerReadyMessage>,
) -> Result<(), BevyError> {
    if let Some(state) = asset_server.get_load_state(folder_handle.0.id()) {
        match state {
            LoadState::Loaded => {
                if let Some(loaded_folder) = loaded_folders.get(folder_handle.0.id()) {
                    define_characters_map(commands, configs, loaded_folder)?;
                    ev_writer.write(ControllerReadyMessage(Controller::Character));
                    controller_state.set(CharacterControllerState::Idle);
                    info!("character controller ready");
                } else {
                    return Err(
                        anyhow::anyhow!("Error loading character assets").into(),
                    );
                }
            }
            LoadState::Failed(e) => {
                return Err(
                    anyhow::anyhow!("Error loading character assets: {}", e.to_string()).into(),
                );
            }
            _ => {}
        }
    }
    Ok(())
}
fn import_characters(mut commands: Commands, asset_server: Res<AssetServer>) {
    let loaded_folder = asset_server.load_folder(CHARACTERS_ASSET_PATH);
    commands.insert_resource(HandleToCharactersFolder(loaded_folder));
}
fn wait_trigger(
    mut msg_reader: MessageReader<ControllersSetStateMessage>,
    mut controller_state: ResMut<NextState<CharacterControllerState>>,
) {
    for msg in msg_reader.read() {
        controller_state.set(msg.0.into());
    }
}
fn update_characters(
    mut commands: Commands,
    mut character_query: Query<(Entity, &mut CharacterConfig, &mut ImageNode)>,
    ui_root: Single<Entity, With<UiRoot>>,
    sprites: Res<CharactersResource>,
    mut configs: ResMut<Configs>,
    mut fading_characters: ResMut<FadingCharacters>,
    mut moving_characters: ResMut<MovingCharacters>,
    mut character_change_message: MessageReader<CharacterChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
    images: Res<Assets<Image>>,
) -> Result<(), BevyError> {
    
    for msg in character_change_message.read() {
        let character_config = configs.0.get_mut(&msg.character).context(format!("Character config not found for {}", &msg.character))?;
        match &msg.operation {
            CharacterOperation::Spawn(info) => {
                let emotion = if let Some(e) = &info.emotion { e.to_owned() } else { character_config.emotion.clone() };
                character_config.emotion = emotion.clone();
                if let Some(_) = character_query.iter_mut().find(|entity| entity.1.name == character_config.name) {
                    warn!("Another instance of the character is already in the World!");
                }
                spawn_character(&mut commands, character_config.clone(), &sprites, info.fading, &mut fading_characters, &ui_root, &images, info.position.clone())?;
                if info.fading {
                    game_state.blocking = true;
                }
            },
            CharacterOperation::EmotionChange(emotion) => {
                if !character_config.emotions.contains(&emotion) {
                    return Err(anyhow::anyhow!("Character does not have {} emotion!", emotion).into());
                }
                let mut entity = match character_query.iter_mut().find(|entity| entity.1.name == character_config.name) {
                    Some(e) => e,
                    None => {
                        let warn_message = format!("Character {} not found in the World!", character_config.name);
                        warn!(warn_message);
                        return Ok(());
                    }
                };
                change_character_emotion(&mut entity.2, &sprites, emotion, character_config)?;
            },
            CharacterOperation::Despawn(fading) => {
                if *fading {
                    for entity in character_query.iter().filter(|c| c.1.name == character_config.name) {
                        fading_characters.0.push((entity.0, -0.01, true));
                    }
                    game_state.blocking = true;
                } else {
                    for entity in character_query.iter().filter(|c| c.1.name == character_config.name) {
                        commands.entity(entity.0).despawn();
                    }
                }
            },
            CharacterOperation::Look(direction) => {
                for (_, _, mut image) in character_query.iter_mut().filter(|c| c.1.name == character_config.name) {
                    image.flip_x = direction == &CharacterDirection::Left;
                }
            },
            CharacterOperation::Move(position) => {
                for (entity, _, _) in character_query.iter_mut().filter(|c| c.1.name == character_config.name) {
                    let target_position = position.to_percentage_value();
                    moving_characters.0.push((entity, target_position));
                    game_state.blocking = true;
                }
            }
        }
    }

    Ok(())
}
