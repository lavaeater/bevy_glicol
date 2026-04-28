# bevy_glicol — CLAUDE.md

## Repo structure

```
bevy_glicol/          ← Bevy plugin wrapping the Glicol audio engine
bevy_glicol_ui/       ← Bevy-native UI for editing Glicol graphs (active work)
tui_glicol/           ← ABANDONED TUI prototype (ratatui), do not continue
lava_ui_builder/      ← Reusable Bevy 0.18 UI builder library (also in this repo)
glicol/               ← Upstream Glicol engine (git submodule)
roadmap.md            ← Full 6-phase plan for bevy_glicol_ui
```

## Key decisions

- The TUI (`tui_glicol`) is superseded. All UI work goes in `bevy_glicol_ui`.
- `bevy_glicol_ui` is a **separate crate** from `bevy_glicol` (not a feature flag).
- `.glicol` is the canonical interchange format; `GlicolGraph` round-trips to/from it.
- Bevy version: **0.18**. Event system is observer-based (`Commands::trigger` + `On<T>`), NOT `EventReader`/`EventWriter`. Asset events use `MessageReader<AssetEvent<T>>`.
- `BorderRadius` is a **field of `Node`**, not a standalone `Component`.
- `BorderColor::all(color)` is the correct constructor.
- `GlicolEngine` lives at `bevy_glicol::prelude::GlicolEngine`.
- `GlicolPara` is generic: always use `GlicolPara<String>` for owned values.
- `LoadContext::path()` returns `&AssetPath<'static>`; get the `Path` via `.path()`.
- `AssetLoader` impls must `#[derive(TypePath)]`.

## bevy_glicol_ui — current state

### Phase 1 ✅ complete
- `src/asset.rs` — `GlicolSource` asset + `GlicolSourceLoader`, `parse_glicol()` parser,
  `LoadGlicolFile` / `SaveGlicolFile` observer events, `on_glicol_asset_loaded` system.
- Parser handles: `~name:` / `o:` / `out:` labels, `>>` chains, continuation lines (`>>`
  at start of line joins to previous), inline `//` comments, `\sample`, `~ref`, numeric params.
  Patterns and rhai blocks are skipped (preserved only in `GlicolSource::raw`).
- On load: raw code is pushed straight to `GlicolEngine` (bypasses graph for fidelity),
  graph is parsed for visualisation.

### Phase 2 ✅ complete
- `src/ui/mod.rs` — `GlicolUiPlugin`, `GlicolUiConfig` (startup file), `UiState` (selected node).
- `src/ui/node_card.rs` — `NodeCard` component, `NodeInputAnchor` / `NodeOutputAnchor` dots,
  `spawn_node_card()` helper. Cards: accent stripe by category, border highlight on selection,
  param summary line.
- `src/ui/graph_view.rs` — `compute_depths` (longest-path), `layout_graph` (column layout),
  `rebuild_graph_cards` (despawn+respawn on graph change), `draw_connection_arrows` (cubic
  bezier via Gizmos), `handle_card_clicks` (click → `UiState::selected_node`).
- Category colours: Oscillator=amber, Filter=blue, Effect=purple, Modulator=green,
  Math=grey, IO=teal, Utility=slate.
- `examples/graph_viewer.rs` — runnable demo, press **L** to cycle patches.

### Next up: checkbox widget in lava_ui_builder
User asked for a `checkbox` widget (markdown `- [ ]` style, toggleable) to be added to
`lava_ui_builder`. Has NOT been implemented yet.

Pattern to follow: `Collapsible` in `src/lib.rs` + toggle system in `src/systems.rs`.
Checkbox needs:
- `Checkbox { checked: bool, label: String }` component
- `CheckboxToggle` marker on the clickable box part
- Visual: square box + checkmark (✓ text or coloured fill) + label text
- System: `toggle_checkbox` in `systems.rs`, reacts to `Interaction::Pressed` on `CheckboxToggle`
- Register in `LavaUiPlugin`
- Bundle-function API: `checkbox(label, checked, theme)` in `lib.rs`
- Imperative API: `UIBuilder::add_checkbox(label, checked)` in `builder.rs`

### Phase 3 (next after checkbox)
Parameter editing panel in the right sidebar — click a node card → show its params
in the side panel with editable fields (number input, reference picker).

## Run the demo

```bash
cargo run --example graph_viewer -p bevy_glicol_ui
# Press L to cycle through bundled patches
```

## Assets

```
bevy_glicol_ui/assets/glicols/   — test.glicol, test2.glicol, synth.glicol, SolsticeStream2023.glicol
bevy_glicol_ui/assets/samples/   — ~50 .wav drum/synth samples
```
