use std::collections::HashMap;
use bevy::asset::{LoadState, LoadedFolder};
use bevy::image::TRANSPARENT_IMAGE_HANDLE;
use bevy::prelude::*;
use bevy::{app::{App, Plugin}, asset::{AssetServer, Handle}};
use anyhow::Context;

use crate::VisualNovelState;
use crate::compiler::controller::{Controller, ControllerReadyMessage, ControllersSetStateMessage, SabiState, UiRoot};

const BACKGROUND_Z_INDEX: i32 = 1;
const BACKGROUNDS_ASSET_PATH: &str   = "sabi/backgrounds";

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
enum BackgroundControllerState {
    /// During Idle state, [BackgroundController] waits for a [ControllersSetStateMessage]
    #[default]
    Idle,
    /// During Loading state, [BackgroundController] loads and wait for assets folder to be completely loaded
    Loading,
    /// In Running state [BackgroundController] handles BackgroundChangeMessage
    Running,
}

impl From<SabiState> for BackgroundControllerState {
    fn from(value: SabiState) -> Self {
        match value {
            SabiState::Idle => BackgroundControllerState::Idle,
            SabiState::WaitingForControllers => BackgroundControllerState::Loading,
            SabiState::Running => BackgroundControllerState::Running,
        }
    }
}

/* Components */
#[derive(Component)]
pub(crate) struct BackgroundNode;
#[derive(Component)]
pub(crate) struct NextBackground;

/* Resources */
/// Resource used to reference the [Handle] to [LoadedFolder] of backgrounds.
#[derive(Resource)]
struct HandleToBackgroundsFolder(Handle<LoadedFolder>);
/// Resource to map [`Handle<Image>`] of background images to background asset names.
#[derive(Resource)]
struct BackgroundImages(HashMap::<String, Handle<Image>>);
#[derive(Resource, Default)]
struct Dissolving(Option<f32>);
#[derive(Resource, Default)]
struct Sliding(BackgroundDirection);

/* Messages */
/// Message used to instruct [BackgroundController] to change current background.
#[derive(Message)]
pub(crate) struct BackgroundChangeMessage {
    pub operation: BackgroundOperation,
}

/* Custom Types */
#[derive(Debug, Clone)]
pub(crate) enum BackgroundOperation {
    ChangeTo(String),
    DissolveTo(Option<String>),
    SlideTo(BackgroundDirection),
}

#[derive(Debug, Clone, Default)]
pub(crate) enum BackgroundDirection {
    #[default]
    North,
    South,
    East,
    West,
}

pub(crate) struct BackgroundController;
impl Plugin for BackgroundController {
    fn build(&self, app: &mut App) {
        app.add_message::<BackgroundChangeMessage>()
            .init_state::<BackgroundControllerState>()
            .init_resource::<Dissolving>()
            .add_systems(Update, check_state_change)
            .add_systems(OnEnter(BackgroundControllerState::Loading), import_backgrounds_folder)
            .add_systems(Update, check_loading_state.run_if(in_state(BackgroundControllerState::Loading)))
            .add_systems(Update, (
                update_background,
                run_dissolving_animation,
                run_sliding_animation,
            ).run_if(in_state(BackgroundControllerState::Running)));
    }
}

/// System to check loading state of assets.
/// When finished, it spawns a [Node] with an empty [ImageNode] in which [BackgroundController] will spawn
/// next backgrounds. This entity is marked with [BackgroundNode] marker
fn check_loading_state(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    folder_handle: Res<HandleToBackgroundsFolder>,
    ui_root: Option<Single<Entity, With<UiRoot>>>,
    mut controller_state: ResMut<NextState<BackgroundControllerState>>,
    mut msg_writer: MessageWriter<ControllerReadyMessage>,
) -> Result<(), BevyError> {
    let mut background_sprites: HashMap<String, Handle<Image>>= HashMap::new();

    if let Some(state) = asset_server.get_load_state(folder_handle.0.id()) {
        match state {
            LoadState::Loaded => {
                if let Some(loaded_folder) = loaded_folders.get(folder_handle.0.id()) {
                    for handle in &loaded_folder.handles {
                        let path = handle.path()
                            .context("Error retrieving background path")?;
                        let filename = path.path().file_stem()
                            .context("Background file has no name")?
                            .to_string_lossy()
                            .to_string();
                        background_sprites.insert(filename, handle.clone().typed());
                    }
                    commands.insert_resource(BackgroundImages(background_sprites));
                } else {
                    return Err(anyhow::anyhow!("Could not find background loaded folder!").into());
                }

                /* Background Setup */
                let ui_root = ui_root.context("Cannot find UiRoot node in the World")?;
                commands.entity(ui_root.entity()).with_child((
                    ImageNode::default(),
                    Node {
                        width: percent(100.),
                        height: percent(100.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Transform::default(),
                    ZIndex(BACKGROUND_Z_INDEX),
                    BackgroundNode,
                    DespawnOnEnter(SabiState::Idle),
                ));
                controller_state.set(BackgroundControllerState::Idle);
                msg_writer.write(ControllerReadyMessage(Controller::Background));
                info!("background controller ready");
            },
            LoadState::Failed(e) => {
                return Err(anyhow::anyhow!("Error loading background assets: {}", e.to_string()).into());
            }
            _ => {}
        }
    }
    Ok(())
}
/// Initiate import procedure and insert [bevy::asset::LoadedFolder] handle into [HandleToBackgroundsFolder] resource.
/// Currently only "backgrounds" folder in bevy "assets" root is supported
fn import_backgrounds_folder(mut commands: Commands, asset_server: Res<AssetServer>){
    let loaded_folder = asset_server.load_folder(BACKGROUNDS_ASSET_PATH);
    commands.insert_resource(HandleToBackgroundsFolder(loaded_folder));
}
/// Checks for state changes from main controller when in [BackgroundControllerState::Idle] state
fn check_state_change(
    mut msg_reader: MessageReader<ControllersSetStateMessage>,
    mut controller_state: ResMut<NextState<BackgroundControllerState>>,
) {
    for msg in msg_reader.read() {
        controller_state.set(msg.0.into());
    }
}
/// Checks for [BackgroundChangeMessage] when in [BackgroundControllerState::Running] state
fn update_background(
    mut background_change_message: MessageReader<BackgroundChangeMessage>,
    background_images: Res<BackgroundImages>,
    mut background_query: Single<(Entity, &mut ImageNode, &mut Node), With<BackgroundNode>>,
    mut vn_state: ResMut<VisualNovelState>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    for msg in background_change_message.read() {
        match &msg.operation {
            BackgroundOperation::ChangeTo(target) => {
                let background_handle = background_images.0.get(target)
                    .with_context(|| format!("Background '{}' does not exist", target))?;
                background_query.1.image = background_handle.clone();
                background_query.2.top = Val::Auto;
                background_query.2.left = Val::Auto;
                background_query.2.bottom = Val::Auto;
                background_query.2.right = Val::Auto;
                info!("[ Change background to '{}']", target);
            },
            BackgroundOperation::DissolveTo(target) => {
                commands.insert_resource(Dissolving(Some(1.)));
                let image_handle = if let Some(target) = target {
                    background_images.0.get(target)
                        .context(format!("Background '{}' does not exist", target))?
                } else {
                    &TRANSPARENT_IMAGE_HANDLE
                };
                commands.entity(background_query.0).with_child((
                    ImageNode {
                        image: image_handle.clone(),
                        color: Color::default().with_alpha(1.),
                        ..default()
                    },
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Transform::default(),
                    NextBackground,
                    DespawnOnEnter(SabiState::Idle),
                ));
                vn_state.blocking = true;
                info!("[ Dissolve background to '{:?}']", target);
            },
            BackgroundOperation::SlideTo(direction) => {
                commands.insert_resource(Sliding(direction.clone()));
                vn_state.blocking = true;
                info!("[ Sliding background to '{:?}']", direction);
            }
        }
    }
    Ok(())
}

/// If a valid [Dissolving] resource is present, this system runs blocks the user input and runs dissolving animation from a background to another one
fn run_dissolving_animation(
    mut commands: Commands,
    mut dissolving: ResMut<Dissolving>,
    mut background_query: Single<&mut ImageNode, With<BackgroundNode>>,
    mut next_background_query: Single<(Entity, &mut ImageNode), (With<NextBackground>, Without<BackgroundNode>)>,
    mut vn_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    
    if let Some(alpha) = &mut dissolving.0 {
        background_query.color.set_alpha(alpha.clone());
        next_background_query.1.color.set_alpha(1. - alpha.clone());
        *alpha -= 0.005;
        if *alpha <= 0. {
            commands.insert_resource(Dissolving(None));
            background_query.image = next_background_query.1.image.clone();
            background_query.color.set_alpha(1.);
            commands.entity(next_background_query.0).despawn();
            vn_state.blocking = false;
        }
    }
    
    Ok(())
}

/// If a [Sliding] resource is set, this system blocks the user input and runs the sliding animation of the background
fn run_sliding_animation(
    mut commands: Commands,
    sliding: Option<ResMut<Sliding>>,
    mut background_query: Single<&mut Node, With<BackgroundNode>>,
    mut vn_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    
    if let Some(sliding) = sliding {
        vn_state.blocking = true;
        let parameter: &mut Val = match &sliding.0 {
            BackgroundDirection::North => &mut background_query.bottom,
            BackgroundDirection::East  => &mut background_query.left,
            BackgroundDirection::South => &mut background_query.top,
            BackgroundDirection::West  => &mut background_query.right,
        };
        *parameter = match parameter {
            Val::Percent(val) => Val::Percent(val.clone() + 0.5),
            _ => Val::Percent(0.),
        };
        if let Val::Percent(val) = parameter {
            if val.clone() > 100. {
                commands.remove_resource::<Sliding>();
                vn_state.blocking = false;
            }
        }
    }
    
    Ok(())
}
