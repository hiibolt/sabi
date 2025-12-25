use std::{any::TypeId, collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use bevy::{asset::{LoadState, LoadedFolder}, prelude::*, window::PrimaryWindow};
use serde::Deserialize;

use crate::{VisualNovelState, actor::operations::{apply_alpha, change_character_emotion, move_characters, spawn_actor}, compiler::controller::{Controller, ControllerReadyMessage, SabiState, ControllersSetStateMessage}};
use crate::compiler::controller::UiRoot;

pub const INVISIBLE_LEFT_PERCENTAGE: f32 = -40.;
pub const FAR_LEFT_PERCENTAGE: f32 = 5.;
pub const FAR_RIGHT_PERCENTAGE: f32 = 65.;
pub const LEFT_PERCENTAGE: f32 = 20.;
pub const CENTER_PERCENTAGE: f32 = 35.;
pub const RIGHT_PERCENTAGE: f32 = 50.;
pub const INVISIBLE_RIGHT_PERCENTAGE: f32 = 140.;
const CHARACTERS_ASSET_PATH: &str = "sabi/characters";
const ANIMATIONS_ASSET_PATH: &str = "sabi/animations";

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
pub(crate) struct CharacterConfig {
    pub name: String,
    pub outfit: String,
    pub emotion: String,
    pub description: String,
    pub emotions: Vec<String>,
    pub outfits: Vec<String>,
}
#[derive(Component, Debug, Default, Asset, TypePath, Deserialize, Clone)]
pub(crate) struct AnimationConfig {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub fps: usize,
    pub rows: usize,
    pub columns: usize,
    pub start_index: usize,
    pub end_index: usize,
    pub once: bool,
}
#[derive(Component, Debug, Asset, TypePath, Deserialize, Clone)]
pub enum ActorConfig {
    Character(CharacterConfig),
    Animation(AnimationConfig),
}

#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) enum ActorPosition {
    Character(CharacterPosition),
    Animation(AnimationPosition),
}

#[derive(Component, Default, Debug, Clone, PartialEq)]
pub(crate) enum CharacterPosition {
    #[default]
    Center,
    FarLeft,
    FarRight,
    Left,
    Right,
    InvisibleLeft,
    InvisibleRight,
}

#[derive(Component, Default, Debug, Clone, PartialEq)]
pub(crate) enum AnimationPosition {
    #[default]
    Center,
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl From<AnimationPosition> for (f32, f32) {
    fn from(value: AnimationPosition) -> Self {
        match value {
            AnimationPosition::TopLeft     => { (15., 85.) },
            AnimationPosition::Top         => { (50., 85.) },
            AnimationPosition::TopRight    => { (85., 85.) },
            AnimationPosition::Left        => { (15., 50.) },
            AnimationPosition::Center      => { (50., 50.) },
            AnimationPosition::Right       => { (85., 50.) },
            AnimationPosition::BottomLeft  => { (15., 15.) },
            AnimationPosition::Bottom      => { (50., 15.) },
            AnimationPosition::BottomRight => { (85., 15.) },
        }
    }
}

impl TryFrom<&str> for AnimationPosition {
    type Error = std::io::Error;
    
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "top left" => Ok(AnimationPosition::TopLeft),
            "top" => Ok(AnimationPosition::Top),
            "top right" => Ok(AnimationPosition::TopRight),
            "left" => Ok(AnimationPosition::Left),
            "center" => Ok(AnimationPosition::Center),
            "right" => Ok(AnimationPosition::Right),
            "bottom left" => Ok(AnimationPosition::BottomLeft),
            "bottom" => Ok(AnimationPosition::Bottom),
            "bottom right" => Ok(AnimationPosition::BottomRight),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unexpected position: {:?}", other),
            ))
        }
    }
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

#[derive(Component)]
pub(crate) struct AnimationTimer(pub Timer);

/* Resources */
#[derive(Resource)]
struct HandleToCharactersFolder(Handle<LoadedFolder>);
#[derive(Resource)]
struct HandleToAnimationsFolder(Handle<LoadedFolder>);

#[derive(Resource, Default)]
pub(crate) struct ActorsResource(pub ActorSprites);

#[derive(Resource, Default, Debug)]
struct ActorsConfigs(ActorsConfig);

#[derive(Resource, Default)]
struct CharFolderLoaded(pub bool);
#[derive(Resource, Default)]
struct AnimFolderLoaded(pub bool);

#[derive(Resource, Default)]
pub(crate) struct FadingActors(pub Vec<(Entity, f32, bool)>); // entity, alpha_step, to_despawn
#[derive(Resource, Default)]
pub(crate) struct MovingActors(pub Vec<(Entity, f32)>); // entity, target_position

/* Custom types */
#[derive(Hash, Eq, PartialEq, Debug)]
pub(crate) enum SpriteIdentifier {
    Character(SpriteKey),
    Animation(String),
}
pub(crate) type ActorSprites = HashMap<SpriteIdentifier, Handle<Image>>;
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub(crate) struct SpriteKey {
    pub character: String,
    pub outfit: String,
    pub emotion: String,
}
type CharacterSprites = HashMap<SpriteKey, Handle<Image>>;
type AnimationSprites = HashMap<String, Handle<Image>>;
type ActorsConfig = HashMap<String, ActorConfig>;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CharacterDirection {
    Left,
    #[default]
    Right
}

impl TryFrom<&str> for CharacterDirection {
    type Error = std::io::Error;
    
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "left" => Ok(CharacterDirection::Left),
            "right" => Ok(CharacterDirection::Right),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unexpected direction: {:?}", other),
            ))
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SpawnInfo {
    pub emotion: Option<String>,
    pub position: Option<ActorPosition>,
    pub direction: CharacterDirection,
    pub fading: bool,
    pub scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActorOperation {
    Spawn(SpawnInfo), 
    EmotionChange(String),
    Despawn(bool), // fading
    Look(CharacterDirection),
    Move(CharacterPosition),
}
pub(crate) enum ActorType {
    Character,
    Animation,
}

/* Messages */
#[derive(Message)]
pub(crate) struct ActorChangeMessage {
    pub r#type: ActorType,
    pub name: String,
    pub operation: ActorOperation,
}
impl ActorChangeMessage {
    pub fn is_blocking(&self) -> bool {
        match &self.operation {
            ActorOperation::Spawn(info) => {
                if info.fading { true } else { false }
            },
            ActorOperation::Despawn(true) => true,
            _ => false
        }
    }
}

pub(crate) struct CharacterController;
impl Plugin for CharacterController {
    fn build(&self, app: &mut App) {
        app.insert_resource(MovingActors::default())
            .insert_resource(FadingActors::default())
            .insert_resource(CharFolderLoaded::default())
            .insert_resource(AnimFolderLoaded::default())
            .insert_resource(ActorsConfigs::default())
            .insert_resource(ActorsResource::default())
            .add_message::<ActorChangeMessage>()
            .init_state::<CharacterControllerState>()
            .add_systems(Update, wait_trigger)
            .add_systems(OnEnter(CharacterControllerState::Loading), import_assets)
            .add_systems(Update, setup.run_if(in_state(CharacterControllerState::Loading)))
            .add_systems(Update, (update_actors, apply_alpha, move_characters)
                .run_if(in_state(CharacterControllerState::Running)))
            .add_systems(OnExit(CharacterControllerState::Running), clean_resources);
    }
}
fn clean_resources(
    mut char_loaded_folder: ResMut<CharFolderLoaded>,
    mut anim_loaded_folder: ResMut<AnimFolderLoaded>,
) {
    char_loaded_folder.0 = false;
    anim_loaded_folder.0 = false;
}
fn define_characters_map(
    commands: &mut Commands,
    actor_config_assets: &Res<Assets<ActorConfig>>,
    loaded_folder: &LoadedFolder,
    actual_configs: &ResMut<ActorsConfigs>,
    sprite_resource: &mut ResMut<ActorsResource>,
) -> Result<(), BevyError> {
    
    let mut characters_sprites = CharacterSprites::new();
    let mut characters_configs = ActorsConfig::new();
    
    let expected_len = PathBuf::from(CHARACTERS_ASSET_PATH).iter().count() + 3;
    
    for handle in &loaded_folder.handles {
        let path = handle.path().context("Error retrieving character asset path")?.path();
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
                actor_config_assets
                    .get(&handle.clone().typed::<ActorConfig>())
                    .context(format!("Failed to retrieve CharacterConfig for '{}'", name))?
                    .clone(),
            );
        }
    }
    for spr in characters_sprites {
        sprite_resource.0.insert(SpriteIdentifier::Character(spr.0), spr.1);
    }
    commands.insert_resource(ActorsConfigs(actual_configs.0.clone().into_iter().chain(characters_configs).collect()));
    Ok(())
}
fn define_animations_map(
    commands: &mut Commands,
    config_res: &Res<Assets<ActorConfig>>,
    loaded_folder: &LoadedFolder,
    actual_configs: &ResMut<ActorsConfigs>,
    sprite_resource: &mut ResMut<ActorsResource>,
) -> Result<(), BevyError> {
    
    let mut animations_configs = ActorsConfig::new();
    let mut animations_sprites = AnimationSprites::new();
    
    for handle in &loaded_folder.handles {
        if handle.type_id() == TypeId::of::<ActorConfig>() {
            let concrete_config = config_res.get(&handle.clone().typed::<ActorConfig>()).context("Could not find concrete configuration")?;
            let config = if let ActorConfig::Animation(config) = concrete_config {
                config
            } else { continue };
            animations_configs.insert(config.name.clone(), ActorConfig::Animation(config.clone()));
        } else {
            let path = handle.path().context("Error retrieving animation asset path")?.path();
            let name: String = path.file_stem().context("Animation file has no name")?.to_string_lossy().to_owned().to_string();
            animations_sprites.insert(name, handle.clone().typed());
        }
    }
    info!("Adding animation resources: {:?}", animations_sprites);
    info!("Adding animation resources: {:?}", animations_configs);
    for anim in animations_sprites {
        sprite_resource.0.insert(SpriteIdentifier::Animation(anim.0), anim.1);
    }
    commands.insert_resource(ActorsConfigs(actual_configs.0.clone().into_iter().chain(animations_configs).collect()));
    
    Ok(())
}
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    folder_char_handle: Res<HandleToCharactersFolder>,
    folder_anim_handle: Res<HandleToAnimationsFolder>,
    actor_config_asset: Res<Assets<ActorConfig>>,
    actual_configs: ResMut<ActorsConfigs>,
    mut sprite_resource: ResMut<ActorsResource>,
    mut char_folder_loaded: ResMut<CharFolderLoaded>,
    mut anim_folder_loaded: ResMut<AnimFolderLoaded>,
    mut controller_state: ResMut<NextState<CharacterControllerState>>,
    mut ev_writer: MessageWriter<ControllerReadyMessage>,
) -> Result<(), BevyError> {
    
    // char folder
    if char_folder_loaded.0 == false {
        if let Some(state) = asset_server.get_load_state(folder_char_handle.0.id()) {
            match state {
                LoadState::Loaded => {
                    if let Some(loaded_folder) = loaded_folders.get(folder_char_handle.0.id()) {
                        define_characters_map(&mut commands, &actor_config_asset, loaded_folder, &actual_configs, &mut sprite_resource)?;
                        char_folder_loaded.0 = true;
                    } else {
                        return Err(anyhow::anyhow!("Error loading character assets").into());
                    }
                }
                LoadState::Failed(e) => {
                    return Err(anyhow::anyhow!("Error loading character assets: {}", e.to_string()).into());
                }
                _ => {}
            }
        }
    }
    
    // animation folder
    if anim_folder_loaded.0 == false {
        if let Some(state) = asset_server.get_load_state(folder_anim_handle.0.id()) {
            match state {
                LoadState::Loaded => {
                    if let Some(loaded_folder) = loaded_folders.get(folder_anim_handle.0.id()) {
                        define_animations_map(&mut commands, &actor_config_asset, loaded_folder, &actual_configs, &mut sprite_resource)?;
                        anim_folder_loaded.0 = true;
                    } else {
                        return Err(anyhow::anyhow!("Error loading animation assets").into());
                    }
                }
                LoadState::Failed(e) => {
                    return Err(anyhow::anyhow!("Error loading animation assets: {}", e.to_string()).into());
                }
                _ => {}
            }
        }
    }
    
    if char_folder_loaded.0 == true && anim_folder_loaded.0 == true {
        ev_writer.write(ControllerReadyMessage(Controller::Character));
        controller_state.set(CharacterControllerState::Idle);
        info!("character controller ready");
    }
    
    Ok(())
}
fn import_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let loaded_char_folder = asset_server.load_folder(CHARACTERS_ASSET_PATH);
    let loaded_anim_folder = asset_server.load_folder(ANIMATIONS_ASSET_PATH);
    commands.insert_resource(HandleToCharactersFolder(loaded_char_folder));
    commands.insert_resource(HandleToAnimationsFolder(loaded_anim_folder));
}
fn wait_trigger(
    mut msg_reader: MessageReader<ControllersSetStateMessage>,
    mut controller_state: ResMut<NextState<CharacterControllerState>>,
) {
    for msg in msg_reader.read() {
        controller_state.set(msg.0.into());
    }
}
fn exec_char_operation(
    character_config: &mut CharacterConfig,
    operation: &ActorOperation,
    character_query: &mut Query<(Entity, &mut CharacterConfig, &mut ImageNode)>,
    mut commands: &mut Commands,
    mut fading_actors: &mut ResMut<FadingActors>,
    moving_actors: &mut ResMut<MovingActors>,
    ui_root: &Single<Entity, With<UiRoot>>,
    game_state: &mut ResMut<VisualNovelState>,
    actor_sprites: &Res<ActorsResource>,
    images: &Res<Assets<Image>>,
    texture_atlases: &mut ResMut<Assets<TextureAtlasLayout>>,
    window: &Window,
) -> Result<(), BevyError> {
    match operation {
        ActorOperation::Spawn(info) => {
            let emotion = if let Some(e) = &info.emotion { e.to_owned() } else { character_config.emotion.clone() };
            character_config.emotion = emotion.clone();
            if let Some(_) = character_query.iter_mut().find(|entity| entity.1.name == character_config.name) {
                warn!("Another instance of the character is already in the World!");
            }
            spawn_actor(&mut commands, ActorConfig::Character(character_config.clone()), &actor_sprites, &mut fading_actors, &ui_root, &images, info.clone(), texture_atlases, &window)?;
            if info.fading {
                game_state.blocking = true;
            }
        },
        ActorOperation::EmotionChange(emotion) => {
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
            change_character_emotion(&mut entity.2, &actor_sprites, emotion, character_config)?;
        },
        ActorOperation::Despawn(fading) => {
            if *fading {
                for entity in character_query.iter().filter(|c| c.1.name == character_config.name) {
                    fading_actors.0.push((entity.0, -0.01, true));
                }
                game_state.blocking = true;
            } else {
                for entity in character_query.iter().filter(|c| c.1.name == character_config.name) {
                    commands.entity(entity.0).despawn();
                }
            }
        },
        ActorOperation::Look(direction) => {
            for (_, _, mut image) in character_query.iter_mut().filter(|c| c.1.name == character_config.name) {
                image.flip_x = direction == &CharacterDirection::Left;
            }
        },
        ActorOperation::Move(position) => {
            for (entity, _, _) in character_query.iter_mut().filter(|c| c.1.name == character_config.name) {
                let target_position = position.to_percentage_value();
                moving_actors.0.push((entity, target_position));
                game_state.blocking = true;
            }
        }
    }
    Ok(())
}
fn exec_anim_operation(
    anim_config: &mut AnimationConfig,
    operation: &ActorOperation,
    character_query: &mut Query<(Entity, &mut CharacterConfig, &mut ImageNode)>,
    mut commands: &mut Commands,
    mut fading_actors: &mut ResMut<FadingActors>,
    moving_actors: &mut ResMut<MovingActors>,
    ui_root: &Single<Entity, With<UiRoot>>,
    game_state: &mut ResMut<VisualNovelState>,
    actor_sprites: &Res<ActorsResource>,
    images: &Res<Assets<Image>>,
    texture_atlases: &mut ResMut<Assets<TextureAtlasLayout>>,
    window: &Window,
) -> Result<(), BevyError> {
    match operation {
        ActorOperation::Spawn(info) => {
            if let Some(_) = character_query.iter_mut().find(|entity| entity.1.name == anim_config.name) {
                warn!("Another instance of the animation is already in the World!");
            }
            spawn_actor(&mut commands, ActorConfig::Animation(anim_config.clone()), &actor_sprites, &mut fading_actors, &ui_root, &images, info.clone(), texture_atlases, &window)?;
            if info.fading {
                game_state.blocking = true;
            }
        },
        ActorOperation::Despawn(fading) => {
            if *fading {
                for entity in character_query.iter().filter(|c| c.1.name == anim_config.name) {
                    fading_actors.0.push((entity.0, -0.01, true));
                }
                game_state.blocking = true;
            } else {
                for entity in character_query.iter().filter(|c| c.1.name == anim_config.name) {
                    commands.entity(entity.0).despawn();
                }
            }
        },
        ActorOperation::Look(direction) => {
            for (_, _, mut image) in character_query.iter_mut().filter(|c| c.1.name == anim_config.name) {
                image.flip_x = direction == &CharacterDirection::Left;
            }
        },
        ActorOperation::Move(position) => {
            for (entity, _, _) in character_query.iter_mut().filter(|c| c.1.name == anim_config.name) {
                let target_position = position.to_percentage_value();
                moving_actors.0.push((entity, target_position));
                game_state.blocking = true;
            }
        },
        other => { return Err(anyhow::anyhow!("Invalid operation on animation {other:?}").into()); }
    }
    Ok(())
}
fn update_actors(
    mut commands: Commands,
    mut character_query: Query<(Entity, &mut CharacterConfig, &mut ImageNode)>,
    ui_root: Single<Entity, With<UiRoot>>,
    actor_sprites: Res<ActorsResource>,
    mut actor_configs: ResMut<ActorsConfigs>,
    mut fading_actors: ResMut<FadingActors>,
    mut moving_actors: ResMut<MovingActors>,
    mut actor_change_message: MessageReader<ActorChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
    images: Res<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    anim_timers: Query<(&mut ImageNode, &AnimationConfig, &mut AnimationTimer), Without<CharacterConfig>>,
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
) -> Result<(), BevyError> {
    
    let window = window.single().context("Could not retrieve window entity")?;
    
    for msg in actor_change_message.read() {
        let actor_config = actor_configs.0.get_mut(&msg.name).context(format!("Actor config not found for {}", &msg.name))?;
        match actor_config {
            ActorConfig::Character(c) => exec_char_operation(c, &msg.operation, &mut character_query, &mut commands, &mut fading_actors, &mut moving_actors, &ui_root, &mut game_state, &actor_sprites, &images, &mut texture_atlases, window)?,
            ActorConfig::Animation(a) => exec_anim_operation(a, &msg.operation, &mut character_query, &mut commands, &mut fading_actors, &mut moving_actors, &ui_root, &mut game_state, &actor_sprites, &images, &mut texture_atlases, window)?,
        }
    }
    
    for (mut image, config, mut timer) in anim_timers {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(atlas) = &mut image.texture_atlas {
                let next_index = atlas.index + 1;
                atlas.index = if next_index > config.end_index {
                    config.start_index
                } else { next_index };
            }
        }
    }

    Ok(())
}
