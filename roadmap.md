# bevy_glicol_ui — Roadmap

A Bevy-native UI for editing, visualising, and playing Glicol audio graphs in games.
The TUI prototype (`tui_glicol`) is superseded; all new work lives here.

---

## North star

A panel (or overlay) that a game can embed to let players / developers build live
Glicol patches at runtime: load `.glicol` files, edit node parameters, connect
nodes visually, and hear results immediately through `bevy_glicol`'s `GlicolEngine`.

---

## Phase 1 — Asset pipeline  ✅ in progress

**Goal:** load `.glicol` files as first-class Bevy assets; round-trip save/load.

| # | Task | Notes |
|---|------|-------|
| 1.1 | `GlicolSource` asset type + `GlicolSourceLoader` | Raw text, `AssetLoader` impl |
| 1.2 | Parse raw code into `GlicolGraph` | Best-effort line parser (see DSL notes below) |
| 1.3 | Serialise `GlicolGraph` back to `.glicol` text | Already have `to_glicol_code()` |
| 1.4 | `LoadGlicolFile` / `SaveGlicolFile` events | Clean API for in-game use |
| 1.5 | Wire loaded code straight to `GlicolEngine` | Bypass graph for raw playback |
| 1.6 | Sample discovery: scan `assets/samples/` | Populate a `SampleLibrary` resource |

### Glicol DSL parse strategy
Lines match one of:
- `~name: <chain>` — named reference node
- `o:` / `o1:` / `out:` — output node
- `// …` — comment (skip)
- blank / whitespace — skip
- continuation lines (no `:` prefix, start with `>>`) — append to previous node

Each chain segment is `node_type param param …` split on `>>`.
Parameters are numbers, `\symbol`, or `~ref`. Patterns (quoted sequences) are
stored as opaque strings. This gives us enough to display and re-emit the code;
full semantic validation is left to the engine.

---

## Phase 2 — Node graph visualisation

**Goal:** render the graph as interactive boxes + arrows inside a Bevy UI panel.

| # | Task | Notes |
|---|------|-------|
| 2.1 | `NodeCard` widget (lava_ui_builder) | Shows node type + params, coloured by category |
| 2.2 | Layout algorithm | Topological sort → column placement; store in `GlicolNode::position` |
| 2.3 | Connection arrows | Bevy `Gizmos` lines between card anchors (or custom mesh strips) |
| 2.4 | Scroll / pan within the graph canvas | Draggable viewport |
| 2.5 | Selected node highlight | Click → `UiState::selected_node` |

---

## Phase 3 — Parameter editing

**Goal:** click a node, edit its parameters, hear changes live.

| # | Task | Notes |
|---|------|-------|
| 3.1 | Parameter detail panel (lava_ui_builder) | Shows name, type, current value |
| 3.2 | Number input field | Text field + enter/esc to confirm |
| 3.3 | Pattern input field | Raw string editor for mini-notation patterns |
| 3.4 | Reference picker | Dropdown of existing `~name` nodes |
| 3.5 | On-confirm: regenerate code + push to engine | `to_glicol_code()` → `GlicolEngine::update_with_code()` |

---

## Phase 4 — Graph editing

**Goal:** add/remove nodes and connections interactively.

| # | Task | Notes |
|---|------|-------|
| 4.1 | Node palette panel (categorised, searchable) | Shows all `NodeRegistry` entries |
| 4.2 | Drag-to-canvas to add node | Spawns `GlicolNode` with default params |
| 4.3 | Draw connection: drag from output anchor to input anchor | Updates `GlicolNode::inputs` |
| 4.4 | Delete node / connection | Right-click or Delete key |
| 4.5 | Undo / redo | `Vec<GlicolGraph>` snapshot stack |

---

## Phase 5 — Sample management

**Goal:** use the bundled `assets/samples/*.wav` from the node graph.

| # | Task | Notes |
|---|------|-------|
| 5.1 | `SampleLibrary` resource: index all `.wav` in `assets/samples/` | Built at startup |
| 5.2 | Register samples with `GlicolEngine` | `engine.add_sample(name, data)` |
| 5.3 | `sp` node type in `NodeRegistry` | `ParameterType::SampleSymbol` |
| 5.4 | Sample browser panel | List + preview (trigger one-shot playback) |

---

## Phase 6 — Polish / game integration

| # | Task | Notes |
|---|------|-------|
| 6.1 | Collapsible / dockable panel | Toggled by a key or game event |
| 6.2 | Theme integration with `LavaTheme` | Expose a `GlicolUiTheme` that maps to `LavaTheme` |
| 6.3 | `GlicolUiPlugin` config: start file, panel position, key binding | `GlicolUiConfig` resource |
| 6.4 | WASM compatibility audit | `bevy_glicol` already has wasm deps; ensure loader works |

---

## Architecture overview

```
bevy_glicol_ui
├── assets/
│   ├── glicols/        ← .glicol patch files (loaded via GlicolSourceLoader)
│   └── samples/        ← .wav files (indexed into SampleLibrary)
└── src/
    ├── lib.rs          ← public re-exports + GlicolUiPlugin
    ├── graph.rs        ← GlicolGraph, GlicolNode, NodeRegistry (Bevy Resources)
    ├── asset.rs        ← GlicolSource asset, GlicolSourceLoader, parse_glicol()
    ├── ui/
    │   ├── mod.rs      ← panel layout, startup system
    │   ├── node_card.rs← NodeCard widget
    │   ├── graph_view.rs← canvas + gizmo arrows
    │   ├── detail.rs   ← parameter editing panel
    │   └── palette.rs  ← node palette sidebar
    └── samples.rs      ← SampleLibrary resource + WAV loader bridge
```

---

## DSL compatibility notes

- `.glicol` is the canonical interchange format; `GlicolGraph` is always derivable
  from it and can always be serialised back to it.
- The graph data model is intentionally a superset of what the parser can
  currently recover (e.g. `position` is UI-only and not round-tripped).
- Nodes using undocumented engine internals (`plate`, `bd`, `hh`, `sp`, `meta`,
  `constsig`, `choose`) are stored as opaque `UnknownNode` entries so they
  survive a load/save round-trip without data loss.
