use crate::character::ActorChangeMessage;
use crate::chat::controller::InfoTextMessage;
use crate::compiler::ast::Statement;
use crate::compiler::calling::{Invoke, InvokeContext, SceneChangeMessage, ActChangeMessage};
use crate::{Cursor, HistoryItem, SabiEnd, ast};
use crate::{BackgroundChangeMessage, CharacterSayMessage, GUIChangeMessage, SabiStart, ScriptId, VisualNovelState};

use std::collections::HashMap;
use std::path::PathBuf;
use bevy::asset::{LoadState, LoadedFolder};
use bevy::color::palettes::css::{BLACK, WHITE};
use bevy::prelude::*;
use anyhow::{Context, Result};

const SCRIPTS_ASSET_PATH: &str = "sabi/acts";

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
pub enum SabiState {
    #[default]
    Idle,
    WaitingForControllers,
    Running,
}

#[derive(Resource, Default)]
struct ControllersReady {
    pub background_controller: bool,
    pub character_controller: bool,
    pub chat_controller: bool,
    pub compiler_controller: bool,
}

/* Components */
#[derive(Component)]
pub struct UiRoot;

/* Messages */
#[derive(Message)]
pub struct ControllersSetStateMessage(pub SabiState);
#[derive(Message)]
pub struct ControllerReadyMessage(pub Controller);

/* Custom Types */
pub enum Controller {
    Background,
    Character,
    Chat,
}

/* Resources */
#[derive(Resource)]
struct HandleToScriptsFolder(Handle<LoadedFolder>);
#[derive(Resource, Default)]
struct ScriptsResource(ScriptsMap);
type ScriptsMap = HashMap<ScriptId, Handle<ast::Act>>;
#[derive(Resource)]
struct CurrentScript(pub ScriptId);

pub struct Compiler;
impl Plugin for Compiler {
    fn build(&self, app: &mut App) {
        app
            .init_state::<SabiState>()
            .init_resource::<ControllersReady>()
            .init_resource::<ScriptsResource>()
            .add_message::<ControllerReadyMessage>()
            .add_message::<ControllersSetStateMessage>()
            .add_message::<SceneChangeMessage>()
            .add_message::<ActChangeMessage>()
            .add_message::<SabiStart>()
            .add_message::<SabiEnd>()
            .add_systems(OnEnter(SabiState::Idle), propagate_state)
            .add_systems(Update, check_start.run_if(in_state(SabiState::Idle)))
            .add_systems(OnEnter(SabiState::WaitingForControllers),
                (
                    spawn_ui_root,
                    propagate_state,
                    import_scripts_folder
                ).chain())
            .add_systems(Update, check_states.run_if(in_state(SabiState::WaitingForControllers)))
            .add_systems(OnEnter(SabiState::Running), trigger_running_controllers)
            .add_systems(Update, (run, handle_scene_changes, handle_act_changes).run_if(in_state(SabiState::Running)));
    }
}
fn trigger_running_controllers(
    mut msg_writer: MessageWriter<ControllersSetStateMessage>,
    mut visual_novel_state: ResMut<VisualNovelState>,
    current_script: Res<CurrentScript>,
    scripts_resource: Res<ScriptsResource>,
    acts: Res<Assets<ast::Act>>,
) -> Result<(), BevyError> {
    let act_handle = scripts_resource.0.get(&current_script.0)
        .context("Could not find script handle")?;
    let act = acts.get(act_handle.id())
        .context("Could not find script element")?;

    visual_novel_state.act = Box::new(act.clone());
    visual_novel_state.statements = Cursor::new(act.scenes.get(&act.entrypoint)
        .context("Error retrieving act entrypoint")?
        .statements.clone());
    visual_novel_state.history.push(HistoryItem::Descriptor(format!("Act: {}\n", act.name)));
    visual_novel_state.history.push(HistoryItem::Descriptor(format!("Scene: {}\n", act.entrypoint)));
    visual_novel_state.blocking = false;

    msg_writer.write(ControllersSetStateMessage(SabiState::Running));
    Ok(())
}
fn propagate_state(
    mut msg_writer: MessageWriter<ControllersSetStateMessage>,
    state: Res<State<SabiState>>,
) {
    msg_writer.write(ControllersSetStateMessage(state.clone()));
}
fn check_start(
    mut commands: Commands,
    mut state: ResMut<NextState<SabiState>>,
    mut msg_reader: MessageReader<SabiStart>
) {
    for msg in msg_reader.read() {
        let script_id = msg.0.clone();
        commands.insert_resource(CurrentScript(script_id));
        state.set(SabiState::WaitingForControllers);
    }
}
fn import_scripts_folder(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    let loaded_folder = asset_server.load_folder(SCRIPTS_ASSET_PATH);
    commands.insert_resource(HandleToScriptsFolder(loaded_folder));
}
fn spawn_ui_root(
    mut commands: Commands,
) {
    commands.spawn((
        Node {
            width: percent(100.),
            height: percent(100.),
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::NONE.into()),
        GlobalTransform::default(),
        UiRoot,
        children![
            (
                Node {
                    width: percent(100.),
                    height: percent(100.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::Srgba(BLACK)),
                children![
                    (
                        Text::from("Loading"),
                        TextColor(Color::Srgba(WHITE)),
                        TextFont {
                            font_size: 40.,
                            ..default()
                        }
                    )
                ],
                ZIndex(100),
                DespawnOnExit(SabiState::WaitingForControllers)
            )
        ]
    ));
}
fn define_script_entry(
    handle: Handle<ast::Act>
) -> Result<(ScriptId, Handle<ast::Act>), BevyError> {
    let path = match handle.path() {
        Some(asset_path) => asset_path.path(),
        None => { return Err(anyhow::anyhow!("Error retrieving script path").into()) }
    };
    
    let expected_len = PathBuf::from(SCRIPTS_ASSET_PATH).iter().count() + 2;

    let script_id = if path.iter().count() == expected_len {
        let chapter = path.components().nth(expected_len - 2)
            .context("Chapter component is not valid")?
            .as_os_str().to_str().context("Could not convert chapter path to os_str")?
            .to_owned();
        let act = path.file_stem().context("Act component is not valid")?
            .to_str().context("Could not convert act path to str")?
            .to_owned();
        ScriptId { chapter, act }
    } else {
        return Err(anyhow::anyhow!("Script path is not correct {}", path.display()).into())
    };
    Ok((script_id, handle))
}
fn check_states(
    mut msg_controller_reader: MessageReader<ControllerReadyMessage>,
    mut controllers_state: ResMut<ControllersReady>,
    mut sabi_state: ResMut<NextState<SabiState>>,
    asset_server: Res<AssetServer>,
    folder_handle: Res<HandleToScriptsFolder>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut scripts_resource: ResMut<ScriptsResource>,
) -> Result<(), BevyError> {
    if !controllers_state.compiler_controller {
        if let Some(state) = asset_server.get_load_state(folder_handle.0.id()) {
            match state {
                LoadState::Loaded => {
                    if let Some(loaded_folder) = loaded_folders.get(folder_handle.0.id()) {
                        for handle in &loaded_folder.handles {
                            let (script_id, entry) = define_script_entry(handle.clone().typed())?;
                            scripts_resource.0.insert(script_id, entry);
                        }
                        info!("Resource complete: {:?}", scripts_resource.0);
                        controllers_state.compiler_controller = true;
                    } else {
                        return Err(anyhow::anyhow!("Could not find script file loaded folder!").into());
                    }
                }
                LoadState::Failed(e) => {
                    return Err(anyhow::anyhow!("Error loading scripts assets: {}", e.to_string()).into());
                }
                _ => {}
            }
        }
    }

    for event in msg_controller_reader.read() {
        let controller = match event.0 {
            Controller::Background => &mut controllers_state.background_controller,
            Controller::Character => &mut controllers_state.character_controller,
            Controller::Chat => &mut controllers_state.chat_controller,
        };
        *controller = true;
    }
    if controllers_state.background_controller
       && controllers_state.character_controller
       && controllers_state.chat_controller
       && controllers_state.compiler_controller {
        sabi_state.set(SabiState::Running);
    }
    Ok(())
}
fn run<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h> (
    mut game_state: ResMut<'a, VisualNovelState>,
    mut character_say_message: MessageWriter<'b, CharacterSayMessage>,
    mut background_change_message: MessageWriter<'c, BackgroundChangeMessage>,
    mut gui_change_message: MessageWriter<'d, GUIChangeMessage>,
    mut scene_change_message: MessageWriter<'e, SceneChangeMessage>,
    mut act_change_message: MessageWriter<'f, ActChangeMessage>,
    mut character_change_message: MessageWriter<'g, ActorChangeMessage>,
    mut info_text_message: MessageWriter<'h, InfoTextMessage>,

    mut state: ResMut<NextState<SabiState>>,
    mut ev_controller_writer: MessageWriter<ControllersSetStateMessage>,
    mut ev_writer: MessageWriter<SabiEnd>,
) -> Result<(), BevyError> {

    if game_state.blocking {
        return Ok(());
    }

    let next_statement = if game_state.rewinding > 0 {
        info!("rewinding {}", game_state.rewinding);
        game_state.rewinding -= 1;
        let next_statement = match game_state.statements.prev() {
            Some(Statement::TextItem(item)) => Some(Statement::TextItem(item)),
            Some(Statement::Stage(_)) => {
                game_state.statements.find_previous()
            },
            // todo: Statement::Code currently not handled
            // is history field needed?
            _ => { None }
        };
        if let Some(_) = &next_statement {
            let _ = game_state.history.pop();
        }
        next_statement
    } else {
        let next_statement = game_state.statements.next();
        if let Some(stm) = &next_statement {
            game_state.history.push(HistoryItem::Statement(stm.clone()));
        }
        next_statement
    };

    if let Some(statement) = next_statement {
        statement.invoke(InvokeContext {
                game_state: &mut game_state,
                character_say_message: &mut character_say_message,
                background_change_message: &mut background_change_message,
                gui_change_message: &mut gui_change_message,
                scene_change_message: &mut scene_change_message,
                act_change_message: &mut act_change_message,
                actor_change_message: &mut character_change_message,
                info_text_message: &mut info_text_message,
            })
            .context("Failed to invoke statement")?;
    } else {
        info!("Finished scripts!");
        state.set(SabiState::Idle);
        ev_controller_writer.write(ControllersSetStateMessage(SabiState::Idle));
        ev_writer.write(SabiEnd);
    }

    Ok(())
}
fn handle_scene_changes(
    mut scene_change_messages: MessageReader<SceneChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    for msg in scene_change_messages.read() {
        let new_scene = game_state.act.scenes.get(&msg.scene_id)
            .context(format!("Scene '{}' not found in current act", msg.scene_id))?
            .clone();

        info!("Changing to scene: {}", msg.scene_id);
        game_state.scene = new_scene.clone();
        game_state.statements = Cursor::new(game_state.scene.statements.clone());
        game_state.history.push(HistoryItem::Descriptor(format!("Scene {}", new_scene.name)));
        game_state.blocking = false;
        info!("[ Scene changed to '{}' ]", msg.scene_id);
    }

    Ok(())
}
fn handle_act_changes(
    mut act_change_messages: MessageReader<ActChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
    mut current_script: ResMut<CurrentScript>,
    scripts_resource: Res<ScriptsResource>,
    scripts_assets: Res<Assets<ast::Act>>,
) -> Result<(), BevyError> {
    for msg in act_change_messages.read() {
        current_script.0.act = msg.act_id.clone();
        let act_handle = scripts_resource.0.get(&current_script.0).context(format!("Could not find act handle for {}", current_script.0.act))?;
        let act = scripts_assets.get(act_handle).context(format!("Could not find act {:?}", act_handle))?;

        info!("Changing to act: {}", current_script.0.act);

        let entrypoint_scene = act.scenes.get(&act.entrypoint)
            .context(format!("Entrypoint scene '{}' not found in act '{}'", act.entrypoint, current_script.0.act))?
            .clone();

        game_state.act = Box::new(act.clone());
        game_state.scene = entrypoint_scene;
        game_state.statements = Cursor::new(game_state.scene.statements.clone());
        game_state.history.push(HistoryItem::Descriptor(format!("Act {}", act.name)));
        game_state.blocking = false;
        info!("[ Act changed to '{}' ]", msg.act_id);
    }

    Ok(())
}
