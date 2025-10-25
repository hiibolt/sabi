use std::collections::HashMap;

use anyhow::{Result, Context};
use bevy::{asset::{LoadState, LoadedFolder}, prelude::*};
use serde::Deserialize;

use crate::{character::character_operations::{apply_alpha, change_character_emotion, spawn_character}, compiler::controller::{Controller, ControllerReadyMessage, TriggerControllersMessage}, ChatScrollStopwatch, GUIScrollText, VisualNovelState};
use crate::compiler::controller::UiRoot;

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
enum CharacterControllerState {
    #[default]
    Loading,
    Idle,
    Running,
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

/* Resources */
#[derive(Resource)]
struct HandleToCharactersFolder(Handle<LoadedFolder>);
#[derive(Resource)]
pub struct CharactersResource(pub CharacterSprites);
#[derive(Resource)]
struct Configs(CharactersConfig);
#[derive(Resource, Default)]
pub struct FadingCharacters(pub Vec<(Entity, f32, bool)>); // entity, alpha_step, to_despawn

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
pub enum CharacterOperation {
    Spawn(Option<String>, bool), // emotion, fading
    EmotionChange(String),
    Despawn(bool), // fading
}

/* Messages */
#[derive(Message)]
pub struct CharacterChangeMessage {
    pub character: String,
    pub operation: CharacterOperation,
}

impl CharacterChangeMessage {
    pub fn is_blocking(&self) -> bool {
        if let CharacterOperation::Spawn(_, true) = self.operation {
            true
        } else if let CharacterOperation::Despawn(true) = self.operation {
            true
        } else { false }
    }
}

pub struct CharacterController;
impl Plugin for CharacterController {
    fn build(&self, app: &mut App) {
        app.insert_resource(FadingCharacters::default())
            .add_message::<CharacterChangeMessage>()
            .init_state::<CharacterControllerState>()
            .add_systems(OnEnter(CharacterControllerState::Loading), import_characters)
            .add_systems(Update, setup.run_if(in_state(CharacterControllerState::Loading)))
            .add_systems(Update, wait_trigger.run_if(in_state(CharacterControllerState::Idle)))
            .add_systems(Update, (update_characters, apply_alpha).run_if(in_state(CharacterControllerState::Running)));
    }
}
fn define_characters_map(
    mut commands: Commands,
    config_res: Res<Assets<CharacterConfig>>,
    loaded_folder: &LoadedFolder,
) -> Result<(), BevyError> {
    let mut characters_sprites = CharacterSprites::new();
    let mut characters_configs = CharactersConfig::new();
    for handle in &loaded_folder.handles {
        let path = handle
            .path()
            .context("Error retrieving character asset path")?
            .path();
        let name: String = match path.iter().nth(1).map(|s| s.to_string_lossy().into()) {
            Some(name) => name,
            None => continue,
        };
        if path.iter().count() == 4 {
            let outfit = match path.iter().nth(2).map(|s| s.to_string_lossy().into()) {
                Some(outfit) => outfit,
                None => continue,
            };
            let emotion = match path.iter().nth(3) {
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
        } else if path.iter().count() == 3 {
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
    let loaded_folder = asset_server.load_folder("characters");
    commands.insert_resource(HandleToCharactersFolder(loaded_folder));
}
fn wait_trigger(
    mut msg_reader: MessageReader<TriggerControllersMessage>,
    mut controller_state: ResMut<NextState<CharacterControllerState>>,
) {
    if msg_reader.read().count() > 0 {
        controller_state.set(CharacterControllerState::Running);
    }
}
fn update_characters(
    mut commands: Commands,
    mut character_query: Query<(Entity, &mut CharacterConfig, &mut ImageNode)>,
    ui_root: Single<Entity, With<UiRoot>>,
    sprites: Res<CharactersResource>,
    mut configs: ResMut<Configs>,
    mut fading_characters: ResMut<FadingCharacters>,
    mut character_change_message: MessageReader<CharacterChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
    images: Res<Assets<Image>>,

    _text_object_query: Query<(&mut Text, &mut GUIScrollText)>,
    _scroll_stopwatch: ResMut<ChatScrollStopwatch>,
) -> Result<(), BevyError> {
    for msg in character_change_message.read() {
        let character_config = configs.0.get_mut(&msg.character).context(format!("Character config not found for {}", &msg.character))?;
        match &msg.operation {
            CharacterOperation::Spawn(emotion, fading) => {
                let emotion = if let Some(e) = emotion { e } else { &character_config.emotion };
                character_config.emotion = emotion.clone();
                if let Some(_) = character_query.iter_mut().find(|entity| entity.1.name == character_config.name) {
                    warn!("Another instance of the character is already in the World!");
                }
                spawn_character(&mut commands, character_config.clone(), &sprites, fading, &mut fading_characters, &ui_root, &images)?;
                if *fading {
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
            }
        }
    }

    Ok(())
}
