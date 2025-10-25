use crate::character::CharacterChangeMessage;
use crate::compiler::calling::{Invoke, InvokeContext, SceneChangeMessage, ActChangeMessage};
use crate::{BackgroundChangeMessage, CharacterSayMessage, GUIChangeMessage, VisualNovelState};
use crate::compiler::ast::{build_scenes, Acts, Rule, SabiParser};
use std::path::PathBuf;
use bevy::prelude::*;
use anyhow::{bail, ensure, Context, Result};
use pest::Parser;

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
enum SabiState {
    #[default]
    WaitingForControllers,
    Running,
}

#[derive(Resource, Default)]
struct ControllersReady {
    pub background_controller: bool,
    pub character_controller: bool,
    pub chat_controller: bool,
}

/* Components */
#[derive(Component)]
pub struct UiRoot;

/* Messages */
#[derive(Message)]
pub struct TriggerControllersMessage;
#[derive(Message)]
pub struct ControllerReadyMessage(pub Controller);

/* Custom Types */
pub enum Controller {
    Background,
    Character,
    Chat,
}

pub struct Compiler;
impl Plugin for Compiler {
    fn build(&self, app: &mut App) {
        app
            .init_state::<SabiState>()
            .init_resource::<ControllersReady>()
            .add_message::<ControllerReadyMessage>()
            .add_message::<TriggerControllersMessage>()
            .add_message::<SceneChangeMessage>()
            .add_message::<ActChangeMessage>()
            .add_systems(Startup, (spawn_ui_root, parse))
            .add_systems(Update, check_states.run_if(in_state(SabiState::WaitingForControllers)))
            .add_systems(Update, (run, handle_scene_changes, handle_act_changes).run_if(in_state(SabiState::Running)));
    }
}
fn spawn_ui_root(
    mut commands: Commands,
) {
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::NONE.into()),
        UiRoot,
    ));
}
fn check_states(
    mut msg_controller_reader: MessageReader<ControllerReadyMessage>,
    mut controllers_state: ResMut<ControllersReady>,
    mut msg_writer: MessageWriter<TriggerControllersMessage>,
    mut sabi_state: ResMut<NextState<SabiState>>,
) {
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
       && controllers_state.chat_controller {
        msg_writer.write(TriggerControllersMessage);
        sabi_state.set(SabiState::Running);
    }
}

fn parse_direntry ( 
    acts: &mut Acts,
    dir_entry: std::fs::DirEntry
) -> Result<()> {
    let file_type = dir_entry.file_type()
        .context("Couldn't get file type!")?;
    
    if file_type.is_file() {
        let file_path = dir_entry.path();
        ensure!(file_path.extension().map_or(false, |ext| ext == "sabi"), "Recieved a file that wasn't a `.sabi` file: {:?}", file_path.extension());
        
        // Get the act name from the file stem
        let act_name = dir_entry
            .path()
            .file_stem()
            .context("Invalid script file name")?
            .to_string_lossy()
            .into_owned();
    
        // Compile the act
        info!("Compiling act: {}", act_name);
        let scenes = {
            let script_contents = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read script file: {:?}", file_path))?;
            let scene_pair = SabiParser::parse(Rule::act, &script_contents)
                .with_context(|| format!("Failed to parse script file: {}", act_name))?
                .next()
                .context("Script file is empty")?;
            
            build_scenes(scene_pair)
                .context("Failed to build scenes from AST")?
        };
        
        ensure!(acts.insert(act_name.clone(), Box::new(scenes)).is_none(), "Duplicate act name '{}'", act_name);
        return Ok(());
    }

    if file_type.is_dir() {
        for entry_result in std::fs::read_dir(dir_entry.path())
            .context("Couldn't read directory!")? {
            let entry = entry_result
                .context("Couldn't get directory entry!")?;
            parse_direntry(acts, entry)?;
        }
        return Ok(());
    }

    bail!("Recieved a directory entry that wasn't a file or directory (likely a symlink)!");
}
fn parse ( mut game_state: ResMut<VisualNovelState> ) -> Result<(), BevyError> {
    info!("Starting parsing");
    
    let mut acts: Acts = Acts::new();
    for dir_entry_result in std::fs::read_dir(PathBuf::from(".").join("assets").join("acts"))
        .context("...while trying to read from the scripts directory")?
    {
        let dir_entry = dir_entry_result
            .context("...while trying to read a directory entry in the scripts directory")?;
        parse_direntry(&mut acts, dir_entry)
            .context("...while trying to parse a script file or directory")?;
    }
    
    // Setup entrypoint - use first available act and its entrypoint scene
    let first_act_id = acts.keys().min()
        .context("No acts found! Please ensure you have at least one `.sabi` file in the acts directory.")?
        .clone();
    
    let act = acts.get(&first_act_id)
        .context("Failed to get first act")?
        .clone();
        
    let scene = act.scenes.get(&act.entrypoint)
        .context("Failed to get entrypoint scene")?
        .clone();
    
    game_state.acts = acts;
    game_state.act = act.clone();
    game_state.scene = scene;
    game_state.statements = game_state.scene.statements.clone().into_iter();
    game_state.blocking = false;
    
    info!("Completed pre-compilation successfully - starting with act '{}', scene '{}'", first_act_id, act.entrypoint);
    
    Ok(())
}

fn run<'a, 'b, 'c, 'd, 'e, 'f, 'g> (
    mut game_state: ResMut<'a, VisualNovelState>,
    
    mut character_say_message: MessageWriter<'b, CharacterSayMessage>,
    mut background_change_message: MessageWriter<'c, BackgroundChangeMessage>,
    mut gui_change_message: MessageWriter<'d, GUIChangeMessage>,
    mut scene_change_message: MessageWriter<'e, SceneChangeMessage>,
    mut act_change_message: MessageWriter<'f, ActChangeMessage>,
    mut character_change_message: MessageWriter<'g, CharacterChangeMessage>,

) -> Result<(), BevyError> {
    if game_state.blocking {
        return Ok(());
    }

    if let Some(statement) = game_state.statements.next() {
        statement.invoke(InvokeContext {
                game_state: &mut game_state,
                character_say_message: &mut character_say_message,
                background_change_message: &mut background_change_message,
                gui_change_message: &mut gui_change_message,
                scene_change_message: &mut scene_change_message,
                act_change_message: &mut act_change_message,
                character_change_message: &mut character_change_message,
            })
            .context("Failed to invoke statement")?;
    }

    Ok(())
}

fn handle_scene_changes(
    mut scene_change_messages: MessageReader<SceneChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    for msg in scene_change_messages.read() {
        let new_scene = game_state.act.scenes.get(&msg.scene_id)
            .with_context(|| format!("Scene '{}' not found in current act", msg.scene_id))?
            .clone();
        
        info!("Changing to scene: {}", msg.scene_id);
        game_state.scene = new_scene;
        game_state.statements = game_state.scene.statements.clone().into_iter();
        game_state.blocking = false;
        info!("[ Scene changed to '{}' ]", msg.scene_id);
    }

    Ok(())
}

fn handle_act_changes(
    mut act_change_messages: MessageReader<ActChangeMessage>,
    mut game_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    for msg in act_change_messages.read() {
        let new_act = game_state.acts.get(&msg.act_id)
            .with_context(|| format!("Act '{}' not found", msg.act_id))?
            .clone();
        
        info!("Changing to act: {}", msg.act_id);
        
        let entrypoint_scene = new_act.scenes.get(&new_act.entrypoint)
            .with_context(|| format!("Entrypoint scene '{}' not found in act '{}'", new_act.entrypoint, msg.act_id))?
            .clone();
        
        game_state.act = new_act.clone();
        game_state.scene = entrypoint_scene;
        game_state.statements = game_state.scene.statements.clone().into_iter();
        game_state.blocking = false;
        info!("[ Act changed to '{}', starting at entrypoint scene '{}' ]", msg.act_id, new_act.entrypoint);
    }
    
    Ok(())
}
