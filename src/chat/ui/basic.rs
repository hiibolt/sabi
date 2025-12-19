use bevy::{color::palettes::css::BLACK, prelude::*};
use bevy_ui_widgets::Button;

use crate::{
        chat::{
            GUIScrollText, INFOTEXT_Z_INDEX_INACTIVE, UI_Z_INDEX, controller::{
                InfoTextComponent, InfoTextContainer, MessageText, NameBoxBackground, NameText, TextBoxBackground, UiButtons, VNContainer, VnCommands
            }, ui::FONT_PATH
    },
    compiler::controller::SabiState
};

pub(in crate::chat) fn backplate_container() -> impl Bundle {
    (
        Node {
            width: Val::Vw(70.),
            height: percent(20.),
            margin: UiRect::all(Val::Auto).with_bottom(px(45.)),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Visibility::Hidden,
        ZIndex(UI_Z_INDEX),
        VNContainer,
        DespawnOnEnter(SabiState::Idle)
    )
}

pub(in crate::chat) fn top_section() -> impl Bundle {
    // Needed for horizontal flex,
    // open to modification
    Node::default()
}

pub(in crate::chat) fn namebox() -> impl Bundle {
    (
        ImageNode::default(),
        Node {
            margin: UiRect::default().with_left(px(45.)),
            aspect_ratio: Some(3.),
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Inherited,
        NameBoxBackground,
    )
}

pub(in crate::chat) fn nametext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Node {
            margin: UiRect::default().with_left(px(35.)),
            ..default()
        },
        Text::new("TEST"),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: 30.0,
            ..default()
        },
        NameText
    )
}

pub(in crate::chat) fn textbox() -> impl Bundle {
    (
        ImageNode::default(),
        Node {
            width: percent(100.),
            min_height: percent(100.),
            padding: UiRect {
                top: percent(2.),
                bottom: percent(2.),
                ..UiRect::horizontal(percent(5.))
            },
            ..default()
        },
        ZIndex(UI_Z_INDEX),
        Visibility::Inherited,
        UiButtons::TextBox,
        Button,
        TextBoxBackground,
    )
}

pub(in crate::chat) fn messagetext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Text::new("TEST"),
        GUIScrollText::default(),
        Node::default(),
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: 30.0,
            ..default()
        },
        MessageText
    )
}

pub(in crate::chat) fn infotext_container(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            min_width: percent(100),
            min_height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            ..default()
        },
        ZIndex(INFOTEXT_Z_INDEX_INACTIVE),
        Button,
        UiButtons::InfoText,
        InfoTextContainer,
        DespawnOnExit(SabiState::Running),
        children![
            infotext(asset_server)
        ]
    )
}

fn infotext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Text::new(""),
        GUIScrollText::default(),
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            max_width: percent(70.),
            ..default()
        },
        TextFont {
            font: asset_server.load(FONT_PATH),
            font_size: 40.0,
            ..default()
        },
        TextLayout {
            justify: Justify::Center,
            linebreak: LineBreak::WordBoundary,
        },
        Visibility::Hidden,
        ZIndex(UI_Z_INDEX),
        InfoTextComponent,
    )
}

pub(in crate::chat::ui) fn button(action: UiButtons) -> Result<impl Bundle, BevyError> {
    let (button_text, position_type) = match action {
        UiButtons::OpenHistory => (String::from("History"), PositionType::Relative),
        UiButtons::ExitHistory => (String::from("Close"), PositionType::Absolute),
        UiButtons::Rewind      => (String::from("Rewind"), PositionType::Relative),
        other                  => return Err(anyhow::anyhow!("{:?} is not a valid button!", other).into()),
    };
    
    Ok((
        Node {
            position_type,
            right: percent(2.),
            top: percent(2.),
            border: UiRect::all(px(2)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect { left: px(5), right: px(5), top: px(3), bottom: px(3) },
            ..default()
        },
        BorderColor::all(Color::WHITE),
        BorderRadius::MAX,
        BackgroundColor(Color::Srgba(BLACK)),
        action,
        Button,
        children![
            Text::new(button_text),
            TextShadow::default()
        ]
    ))
}

pub(in crate::chat) fn vn_commands() -> Result<impl Bundle, BevyError> {
    Ok((
        Node {
            position_type: PositionType::Absolute,
            bottom: percent(0.),
            right: percent(0.),
            flex_direction: FlexDirection::Row,
            margin: UiRect::default()
                .with_bottom(percent(1.5))
                .with_right(percent(3.)),
            ..default()
        },
        VnCommands,
        ZIndex(UI_Z_INDEX),
        children![
            button(UiButtons::Rewind)?,
            button(UiButtons::OpenHistory)?,
        ]
    ))
}