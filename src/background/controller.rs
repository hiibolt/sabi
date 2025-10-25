use std::collections::HashMap;

use bevy::asset::{LoadState, LoadedFolder};
use bevy::prelude::*;
use bevy::{app::{App, Plugin}, asset::{AssetServer, Handle}};
use anyhow::Context;

use crate::compiler::controller::{Controller, ControllerReadyMessage, TriggerControllersMessage, UiRoot};

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
enum BackgroundControllerState {
    #[default]
    Loading,
    Idle,
    Running,
}

/* Components */
#[derive(Component)]
pub struct BackgroundNode;

/* Resources */
#[derive(Resource)]
struct HandleToBackgroundsFolder(Handle<LoadedFolder>);
#[derive(Resource)]
struct BackgroundImages(HashMap::<String, Handle<Image>>);

/* Messages */
#[derive(Message)]
pub struct BackgroundChangeMessage {
    pub background_id: String
}

pub struct BackgroundController;
impl Plugin for BackgroundController {
    fn build(&self, app: &mut App) {
        app.add_message::<BackgroundChangeMessage>()
            .init_state::<BackgroundControllerState>()
            .add_systems(OnEnter(BackgroundControllerState::Loading), import_backgrounds)
            .add_systems(Update, setup.run_if(in_state(BackgroundControllerState::Loading)))
            .add_systems(Update, wait_trigger.run_if(in_state(BackgroundControllerState::Idle)))
            .add_systems(Update, update_background.run_if(in_state(BackgroundControllerState::Running)));
    }
}
fn setup(
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
                }

                /* Background Setup */
                let ui_root = ui_root.with_context(|| "Cannot find UiRoot node in the World")?;
                commands.entity(ui_root.entity()).with_child((
                    ImageNode::default(),
                    Node {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Transform::default(),
                    BackgroundNode,
                ));
                controller_state.set(BackgroundControllerState::Idle);
                msg_writer.write(ControllerReadyMessage(Controller::Background));
            },
            LoadState::Failed(e) => {
                return Err(anyhow::anyhow!("Error loading background assets: {}", e.to_string()).into());
            }
            _ => {}
        }
    }
    Ok(())
}
fn import_backgrounds(mut commands: Commands, asset_server: Res<AssetServer>){
    let loaded_folder = asset_server.load_folder("backgrounds");
    commands.insert_resource(HandleToBackgroundsFolder(loaded_folder));
}
fn wait_trigger(
    mut msg_reader: MessageReader<TriggerControllersMessage>,
    mut controller_state: ResMut<NextState<BackgroundControllerState>>,
) {
    if msg_reader.read().count() > 0 {
        controller_state.set(BackgroundControllerState::Running);
    }
}
fn update_background(
    mut background_change_message: MessageReader<BackgroundChangeMessage>,
    background_images: Res<BackgroundImages>,
    mut background_query: Single<&mut ImageNode, With<BackgroundNode>>,
) -> Result<(), BevyError> {
    for msg in background_change_message.read() {
        let background_handle = background_images.0.get(&msg.background_id)
            .with_context(|| format!("Background '{}' does not exist", msg.background_id))?;
        background_query.image = background_handle.clone();
        info!("[ Set background to '{}']", msg.background_id);
    }
    Ok(())
}
