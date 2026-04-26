use bevy::prelude::*;
use crate::graph::NodeCategory;

// ── Colors per node category ──────────────────────────────────────────────────

pub fn category_color(cat: &NodeCategory) -> Color {
    match cat {
        NodeCategory::Oscillator => Color::srgb(0.85, 0.55, 0.15),
        NodeCategory::Filter     => Color::srgb(0.20, 0.50, 0.85),
        NodeCategory::Effect     => Color::srgb(0.60, 0.25, 0.80),
        NodeCategory::Modulator  => Color::srgb(0.25, 0.70, 0.35),
        NodeCategory::Math       => Color::srgb(0.50, 0.50, 0.55),
        NodeCategory::IO         => Color::srgb(0.20, 0.70, 0.65),
        NodeCategory::Utility    => Color::srgb(0.45, 0.45, 0.55),
    }
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component, Clone)]
pub struct NodeCard {
    pub node_id: String,
}

#[derive(Component)]
pub struct NodeOutputAnchor {
    pub node_id: String,
}

#[derive(Component)]
pub struct NodeInputAnchor {
    pub node_id: String,
}

// ── Layout constants ──────────────────────────────────────────────────────────

pub const CARD_W: f32 = 160.0;
pub const CARD_H: f32 = 70.0;
const CARD_R: f32 = 6.0;
const ANCHOR_R: f32 = 5.0;

// ── Spawning ──────────────────────────────────────────────────────────────────

pub fn spawn_node_card(
    commands: &mut Commands,
    parent: Entity,
    node_id: &str,
    label: &str,
    category: &NodeCategory,
    params_line: &str,
    pos: Vec2,
    selected: bool,
) -> Entity {
    let accent = category_color(category);
    let bg = Color::srgba(0.10, 0.10, 0.15, 0.95);
    let border_col = if selected { Color::srgb(1.0, 1.0, 0.3) } else { accent };

    // Card root
    let card = commands.spawn((
        NodeCard { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(pos.x),
            top: Val::Px(pos.y),
            width: Val::Px(CARD_W),
            height: Val::Px(CARD_H),
            flex_direction: FlexDirection::Column,
            padding: UiRect {
                left: Val::Px(6.0), right: Val::Px(6.0),
                top: Val::Px(4.0), bottom: Val::Px(4.0),
            },
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(CARD_R)),
            row_gap: Val::Px(2.0),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border_col),
        Interaction::default(),
    )).id();
    commands.entity(parent).add_child(card);

    // Accent stripe at top
    let stripe = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(3.0),
            border_radius: BorderRadius::top(Val::Px(CARD_R - 2.0)),
            ..default()
        },
        BackgroundColor(accent),
    )).id();
    commands.entity(card).add_child(stripe);

    // Node label (id: type)
    let type_label = commands.spawn((
        Text::new(label),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::WHITE),
    )).id();
    commands.entity(card).add_child(type_label);

    // Parameter summary
    if !params_line.is_empty() {
        let param_label = commands.spawn((
            Text::new(params_line),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgba(0.75, 0.75, 0.75, 0.8)),
        )).id();
        commands.entity(card).add_child(param_label);
    }

    // Input anchor (top-centre dot)
    let in_anchor = commands.spawn((
        NodeInputAnchor { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(CARD_W / 2.0 - ANCHOR_R),
            top: Val::Px(-ANCHOR_R),
            width: Val::Px(ANCHOR_R * 2.0),
            height: Val::Px(ANCHOR_R * 2.0),
            border_radius: BorderRadius::all(Val::Px(ANCHOR_R)),
            ..default()
        },
        BackgroundColor(accent),
    )).id();
    commands.entity(card).add_child(in_anchor);

    // Output anchor (bottom-centre dot)
    let out_anchor = commands.spawn((
        NodeOutputAnchor { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(CARD_W / 2.0 - ANCHOR_R),
            bottom: Val::Px(-ANCHOR_R),
            width: Val::Px(ANCHOR_R * 2.0),
            height: Val::Px(ANCHOR_R * 2.0),
            border_radius: BorderRadius::all(Val::Px(ANCHOR_R)),
            ..default()
        },
        BackgroundColor(accent),
    )).id();
    commands.entity(card).add_child(out_anchor);

    card
}
