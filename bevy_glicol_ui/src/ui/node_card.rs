use bevy::prelude::*;
use crate::graph::NodeCategory;

// ── Colors per node category ──────────────────────────────────────────────────

pub fn category_color(cat: &NodeCategory) -> Color {
    match cat {
        NodeCategory::Oscillator => Color::srgb(0.85, 0.55, 0.15), // amber
        NodeCategory::Filter     => Color::srgb(0.20, 0.50, 0.85), // blue
        NodeCategory::Effect     => Color::srgb(0.60, 0.25, 0.80), // purple
        NodeCategory::Modulator  => Color::srgb(0.25, 0.70, 0.35), // green
        NodeCategory::Math       => Color::srgb(0.50, 0.50, 0.55), // grey
        NodeCategory::IO         => Color::srgb(0.20, 0.70, 0.65), // teal
        NodeCategory::Utility    => Color::srgb(0.45, 0.45, 0.55), // slate
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

/// Marks a UI entity as a node card and carries its graph node ID.
#[derive(Component, Clone)]
pub struct NodeCard {
    pub node_id: String,
}

/// Marks the output anchor point of a node card (bottom-centre).
#[derive(Component)]
pub struct NodeOutputAnchor {
    pub node_id: String,
}

/// Marks the input anchor point of a node card (top-centre).
#[derive(Component)]
pub struct NodeInputAnchor {
    pub node_id: String,
}

// ── Spawning helper ───────────────────────────────────────────────────────────

pub const CARD_W: f32 = 160.0;
pub const CARD_H: f32 = 68.0;
pub const CARD_CORNER: f32 = 6.0;

/// Spawns a node card as a child of `parent` at canvas position `pos`.
/// Returns the spawned entity.
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
    let border_color = if selected {
        Color::srgb(1.0, 1.0, 0.3)
    } else {
        accent
    };

    let card = commands.spawn((
        NodeCard { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(pos.x),
            top: Val::Px(pos.y),
            width: Val::Px(CARD_W),
            height: Val::Px(CARD_H),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect { left: Val::Px(6.0), right: Val::Px(6.0), top: Val::Px(4.0), bottom: Val::Px(4.0) },
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor(border_color),
        BorderRadius::all(Val::Px(CARD_CORNER)),
        Interaction::default(),
    )).id();

    // Accent stripe at top
    let stripe = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(accent),
        BorderRadius::top(Val::Px(CARD_CORNER - 2.0)),
    )).id();
    commands.entity(card).add_child(stripe);

    // Node type label
    let type_label = commands.spawn((
        Text::new(label),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::WHITE),
    )).id();
    commands.entity(card).add_child(type_label);

    // Parameter summary line
    if !params_line.is_empty() {
        let param_label = commands.spawn((
            Text::new(params_line),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgba(0.8, 0.8, 0.8, 0.7)),
        )).id();
        commands.entity(card).add_child(param_label);
    }

    // Input anchor marker (top-center, zero-size, used by gizmo system)
    let in_anchor = commands.spawn((
        NodeInputAnchor { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(CARD_W / 2.0 - 4.0),
            top: Val::Px(-4.0),
            width: Val::Px(8.0),
            height: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(accent),
        BorderRadius::all(Val::Px(4.0)),
    )).id();
    commands.entity(card).add_child(in_anchor);

    // Output anchor marker (bottom-center)
    let out_anchor = commands.spawn((
        NodeOutputAnchor { node_id: node_id.to_string() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(CARD_W / 2.0 - 4.0),
            bottom: Val::Px(-4.0),
            width: Val::Px(8.0),
            height: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(accent),
        BorderRadius::all(Val::Px(4.0)),
    )).id();
    commands.entity(card).add_child(out_anchor);

    commands.entity(parent).add_child(card);
    card
}
