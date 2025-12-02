use anyhow::Context;
use bevy::{color::palettes::css::{BLUE, GRAY, GREEN, PURPLE, RED}, ecs::relationship::RelatedSpawner, prelude::*};
use bevy_ui_widgets::{Button, CoreScrollbarThumb, Scrollbar};
use crate::{VisualNovelState, chat::{GUIScrollText, controller::{CurrentTextBoxBackground, InfoText, MessageText, NameBoxBackground, NameText, TextBoxBackground, UiButtons, VNContainer, VnCommands}}, compiler::{ast::{Evaluate, Statement}, controller::SabiState}};

const UI_Z_INDEX: i32 = 4;

pub(crate) fn backplate_container() -> impl Bundle {
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

pub(crate) fn top_section() -> impl Bundle {
    // Needed for horizontal flex,
    // open to modification
    Node::default()
}

pub(crate) fn namebox() -> impl Bundle {
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

pub(crate) fn nametext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Node {
            margin: UiRect::default().with_left(px(35.)),
            ..default()
        },
        Text::new("TEST"),
        TextFont {
            font: asset_server.load("fonts/ALLER.ttf"),
            font_size: 30.0,
            ..default()
        },
        NameText
    )
}

pub(crate) fn textbox() -> impl Bundle {
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

pub(crate) fn messagetext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Text::new("TEST"),
        GUIScrollText::default(),
        Node::default(),
        TextFont {
            font: asset_server.load("fonts/ALLER.ttf"),
            font_size: 30.0,
            ..default()
        },
        MessageText
    )
}

pub(crate) fn infotext(asset_server: &Res<AssetServer>) -> impl Bundle {
    (
        Text::new(""),
        Node::default(),
        TextFont {
            font: asset_server.load("fonts/ALLER.ttf"),
            font_size: 40.0,
            ..default()
        },
        TextLayout {
            justify: Justify::Center,
            linebreak: LineBreak::WordBoundary,
        },
        ZIndex(UI_Z_INDEX),
        TextColor(Color::Srgba(RED)),
        Visibility::Hidden,
        InfoText,
        DespawnOnEnter(SabiState::Idle)
    )
}

pub(crate) fn vn_commands() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            bottom: percent(0.),
            right: percent(0.),
            border: UiRect::all(px(3.)),
            flex_direction: FlexDirection::Row,
            margin: UiRect::default()
                .with_bottom(percent(1.5))
                .with_right(percent(3.)),
            ..default()
        },
        BorderColor::all(Color::Srgba(GREEN)),
        VnCommands,
        ZIndex(UI_Z_INDEX),
        children![
            rewind_button(),
            history_button(),
        ]
    )
}

fn rewind_button() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Relative,
            border: UiRect::all(px(2)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect { left: px(5), right: px(5), top: px(3), bottom: px(3) },
            ..default()
        },
        BorderColor::all(Color::WHITE),
        BorderRadius::MAX,
        BackgroundColor(Color::Srgba(PURPLE)),
        UiButtons::Rewind,
        Button,
        children![
            Text::new("Rewind"),
            TextShadow::default(),
        ],
    )
}

fn history_button() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Relative,
            border: UiRect::all(px(2)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect { left: px(5), right: px(5), top: px(3), bottom: px(3) },
            ..default()
        },
        BorderColor::all(Color::WHITE),
        BorderRadius::MAX,
        BackgroundColor(Color::Srgba(RED)),
        UiButtons::OpenHistory,
        Button,
        children![
            Text::new("History"),
            TextShadow::default(),
        ],
    )
}
