use sabi::*;
use bevy::{
    prelude::*,
    window::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: String::from("Sabi"),
                    resolution: (1280, 800).into(),
                    present_mode: PresentMode::AutoVsync,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            })
        )
        .add_plugins(SabiPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut msg_writer: MessageWriter<SabiStart>,
    mut user_defined_constants: ResMut<UserDefinedConstants>,
) {
    user_defined_constants.playername = "Test".into();
    // Create our primary camera (which is
    //  necessary even for 2D games)
    commands.spawn(Camera2d::default());
    msg_writer.write(SabiStart(ScriptId { chapter: "examples".into(), act: "animation".into() }));
}