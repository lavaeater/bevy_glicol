use std::collections::HashMap;

use bevy::prelude::*;
use glicol_synth::GlicolPara;

use crate::graph::{GlicolGraph, GlicolNode, NodeCategory, NodeRegistry};
use crate::ui::node_card::{
    NodeCard, NodeInputAnchor, NodeOutputAnchor, CARD_H, CARD_W, spawn_node_card,
};
use crate::ui::UiState;

// ── Layout constants ──────────────────────────────────────────────────────────

const COL_GAP: f32 = 200.0; // horizontal spacing between columns
const ROW_GAP: f32 = 100.0; // vertical spacing between cards in a column
const CANVAS_PAD: f32 = 20.0; // padding around the whole layout

// ── Marker components ─────────────────────────────────────────────────────────

/// Root entity of the graph canvas (the pannable/scrollable area).
#[derive(Component)]
pub struct GraphCanvas;

/// Marks all node card entities so they can be batch-despawned on graph rebuild.
#[derive(Component)]
pub(super) struct GraphCardRoot;

// ── Auto-layout ───────────────────────────────────────────────────────────────

/// Assign a column depth to every node via longest-path from sources.
fn compute_depths(graph: &GlicolGraph) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();

    fn visit(
        id: &str,
        graph: &GlicolGraph,
        depths: &mut HashMap<String, usize>,
        stack: &mut Vec<String>,
    ) -> usize {
        if let Some(&d) = depths.get(id) { return d; }
        if stack.contains(&id.to_string()) { return 0; } // cycle guard
        stack.push(id.to_string());

        let node = match graph.nodes.get(id) { Some(n) => n, None => { stack.pop(); return 0; } };
        let depth = node.inputs.iter()
            .map(|inp| visit(inp, graph, depths, stack) + 1)
            .max()
            .unwrap_or(0);

        stack.pop();
        depths.insert(id.to_string(), depth);
        depth
    }

    let ids: Vec<String> = graph.nodes.keys().cloned().collect();
    let mut stack = Vec::new();
    for id in &ids {
        visit(id, graph, &mut depths, &mut stack);
    }
    depths
}

/// Map every node to a canvas `Vec2` position.
pub fn layout_graph(graph: &GlicolGraph) -> HashMap<String, Vec2> {
    let depths = compute_depths(graph);

    // Group nodes by column depth, sorted for stable output
    let max_depth = depths.values().copied().max().unwrap_or(0);
    let mut columns: Vec<Vec<&str>> = vec![Vec::new(); max_depth + 1];
    let mut sorted_ids: Vec<&str> = graph.nodes.keys().map(String::as_str).collect();
    sorted_ids.sort();
    for id in sorted_ids {
        let d = depths.get(id).copied().unwrap_or(0);
        columns[d].push(id);
    }

    let mut positions = HashMap::new();
    for (col, nodes) in columns.iter().enumerate() {
        let x = CANVAS_PAD + col as f32 * (CARD_W + COL_GAP);
        let total_h = nodes.len() as f32 * (CARD_H + ROW_GAP) - ROW_GAP;
        let start_y = CANVAS_PAD; // top-align columns
        let _ = total_h;
        for (row, id) in nodes.iter().enumerate() {
            let y = start_y + row as f32 * (CARD_H + ROW_GAP);
            positions.insert(id.to_string(), Vec2::new(x, y));
        }
    }
    positions
}

// ── Canvas setup ──────────────────────────────────────────────────────────────

pub fn spawn_graph_canvas(mut commands: Commands) {
    commands.spawn((
        GraphCanvas,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            // Leave 28% on the right for the node list panel
            right: Val::Percent(28.0),
            bottom: Val::Px(0.0),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 1.0)),
    ));
}

// ── Rebuild cards whenever the graph changes ──────────────────────────────────

pub fn rebuild_graph_cards(
    mut commands: Commands,
    graph: Res<GlicolGraph>,
    registry: Res<NodeRegistry>,
    ui_state: Res<UiState>,
    canvas_q: Query<Entity, With<GraphCanvas>>,
    card_roots: Query<Entity, With<GraphCardRoot>>,
) {
    if !graph.is_changed() && !ui_state.is_changed() { return; }

    // Despawn previous card tree
    for e in card_roots.iter() {
        commands.entity(e).despawn();
    }

    let Ok(canvas) = canvas_q.single() else { return };

    // Spawn a fresh container inside the canvas
    let card_root = commands.spawn((
        GraphCardRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    )).id();
    commands.entity(canvas).add_child(card_root);

    if graph.nodes.is_empty() { return; }

    let positions = layout_graph(&graph);

    for (id, node) in &graph.nodes {
        let pos = positions.get(id).copied().unwrap_or(Vec2::ZERO);
        let category = registry
            .get_definition(&node.node_type)
            .map(|d| &d.category)
            .unwrap_or(&NodeCategory::Utility);
        let params_line = format_params_short(&node.parameters);
        let label = format!("{}: {}", id, node.node_type);
        let selected = ui_state.selected_node.as_deref() == Some(id.as_str());

        spawn_node_card(
            &mut commands,
            card_root,
            id,
            &label,
            category,
            &params_line,
            pos,
            selected,
        );
    }
}

fn format_params_short(params: &[GlicolPara<String>]) -> String {
    params.iter().take(3).map(|p| match p {
        GlicolPara::Number(n) => {
            if *n == n.floor() && n.abs() < 10000.0 { format!("{}", *n as i64) }
            else { format!("{:.2}", n) }
        }
        GlicolPara::Reference(r) => format!("~{}", r),
        GlicolPara::SampleSymbol(s) => format!("\\{}", s),
        _ => "…".into(),
    }).collect::<Vec<_>>().join("  ")
}

// ── Connection arrows (Gizmos) ────────────────────────────────────────────────

/// Draw a curved arrow from each output anchor to the input anchor(s) it feeds.
pub fn draw_connection_arrows(
    graph: Res<GlicolGraph>,
    registry: Res<NodeRegistry>,
    out_anchors: Query<(&NodeOutputAnchor, &GlobalTransform)>,
    in_anchors: Query<(&NodeInputAnchor, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    // Build lookup: node_id → output/input anchor world position
    let mut out_pos: HashMap<String, Vec2> = HashMap::new();
    for (anchor, gt) in out_anchors.iter() {
        let p = gt.translation().truncate();
        out_pos.insert(anchor.node_id.clone(), p);
    }
    let mut in_pos: HashMap<String, Vec2> = HashMap::new();
    for (anchor, gt) in in_anchors.iter() {
        let p = gt.translation().truncate();
        in_pos.insert(anchor.node_id.clone(), p);
    }

    // For each node, draw a line from each input's output anchor to this node's input anchor.
    for (node_id, node) in &graph.nodes {
        let Some(&to) = in_pos.get(node_id) else { continue };
        let category = registry
            .get_definition(&node.node_type)
            .map(|d| &d.category)
            .unwrap_or(&NodeCategory::Utility);
        let color = crate::ui::node_card::category_color(category).with_alpha(0.6);

        for input_id in &node.inputs {
            let Some(&from) = out_pos.get(input_id) else { continue };
            // Simple bezier-ish curve approximated as 3 segments
            let cp1 = Vec2::new(from.x, from.y - 30.0);
            let cp2 = Vec2::new(to.x, to.y + 30.0);
            draw_bezier(&mut gizmos, from, cp1, cp2, to, color, 12);
        }
    }
}

/// Quadratic-ish bezier via linear interpolation steps (Bevy Gizmos has no curve API in 0.18).
fn draw_bezier(
    gizmos: &mut Gizmos,
    p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2,
    color: Color,
    steps: usize,
) {
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let q0 = p0.lerp(p1, t);
        let q1 = p1.lerp(p2, t);
        let q2 = p2.lerp(p3, t);
        let r0 = q0.lerp(q1, t);
        let r1 = q1.lerp(q2, t);
        let cur = r0.lerp(r1, t);
        gizmos.line_2d(prev, cur, color);
        prev = cur;
    }
}

// ── Click to select ───────────────────────────────────────────────────────────

pub fn handle_card_clicks(
    cards: Query<(&NodeCard, &Interaction), Changed<Interaction>>,
    mut ui_state: ResMut<UiState>,
) {
    for (card, interaction) in cards.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.selected_node = Some(card.node_id.clone());
        }
    }
}
