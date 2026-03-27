# TUI Glicol

## My Notes
Phase 3 complete — live parameter text input implemented.
- New `GraphParamInput` mode; Tab/Shift-Tab to select param, Enter to edit, type value, Enter to confirm, Esc to cancel.
- Input buffer pre-fills with current param value and shows a `|` cursor while typing.
- GraphNextParam/GraphPrevParam actions now properly wired in app.rs.

A terminal user interface for the [Glicol](https://glicol.org/) music engine, built in Rust with `ratatui` and `cpal`.

Glicol uses a node-based DSL to connect oscillators, samples, beats, effects, and math operations into audio graphs. This TUI aims to make that graph live-editable without writing raw Glicol code.

## Current State (as of 2026-03-27)

### What works

- **Audio engine**: Glicol engine initialised with `cpal`, plays audio on startup (sine 440 Hz default)
- **Sample loading**: WAV samples loaded from `.config/sample-list.json` at startup
- **Keybinding config**: JSON5-based keybinding system with mode-aware dispatch (`Home`, `Graph`, `GraphEditing`)
- **Actions**: `PlayAudio`, `StopAudio`, `UpdateAudioCode(code)`, `SpecialAudio` (loads `SolsticeStream2023.glicol`)
- **Mode switching**: `SwitchMode(Graph)` action exists and sets `App.mode`, but the graph view is not conditionally rendered yet
- **Graph data model**: `Graph` struct with `Node`, `NodeRegistry`, `NodeTypeDefinition` — supports add/remove/connect nodes, parameter validation, AST export via `to_glicol_ast()`, and Glicol DSL export via `to_glicol_code()`
- **Graph-Engine sync**: `Graph::to_glicol_code()` serializes the node graph into Glicol DSL with chain flattening (e.g. `sin 440 >> mul 0.3`). Graph-modifying actions (`GraphAddNode`, `GraphRemoveNode`, `GraphConnectNodes`, `GraphEditParam`) automatically regenerate code and call `engine.update_with_code()`. Engine is initialized from graph on startup.
- **Graph component UI**: Category tabs, node list, node detail panel with parameter display, error bar — all render correctly when nodes exist
- **Graph actions**: Navigation (`j`/`k`), category browsing (`h`/`l`), add/remove nodes, start/stop editing, param navigation (`Tab`/`Shift-Tab`)
- **Log display**: Bottom panel showing recent info/error messages

### What doesn't work yet

- **Mode-conditional rendering**: Pressing `<g>` switches internal mode to `Graph` but `render()` always draws all components + `GraphComponent` (drawn twice — once via `self.components` and once directly). Home view has no way to hide when in Graph mode and vice versa.
- **Reverse sync (engine→graph)**: No path to populate graph from an externally loaded `.glicol` file or `SpecialAudio` code.
- **File I/O**: No save/load for `.glicol` files or any proprietary graph format.
- **Text input for params**: `GraphEditParam` keybinding is hardcoded to example values; no actual text input widget for typing parameter values.

## Plan

### Phase 1 — Mode-conditional rendering & basic navigation

1. Fix `App::render()` to draw `Home` only in `Home` mode and `GraphComponent` only in `Graph`/`GraphEditing` mode (stop double-drawing graph)
2. Verify mode switching works end-to-end (`<g>` to Graph, `<Esc>` to Home)
3. Seed the graph with a simple default patch on startup so there's something to see in Graph mode

### Phase 2 — Graph-Engine round-trip ✅

4. ~~Implement `Graph::to_glicol_code()` — serialize the node graph back into Glicol DSL text~~ ✅
5. ~~Wire up: graph edit -> `to_glicol_code()` -> `engine.update_with_code()` so edits are heard immediately~~ ✅
6. ~~Engine initialized from graph-generated code on startup (graph and engine always in sync)~~ ✅

### Phase 3 — Parameter editing

7. Add a text input widget (inline or popup) for editing parameter values in `GraphEditing` mode
8. Replace hardcoded `GraphEditParam` keybinding with dynamic input that writes to the selected param
9. Support all `ParameterType` variants: `Number`, `Reference`, `Pattern`

### Phase 4 — File I/O

10. **Save as `.glicol`**: write `Graph::to_glicol_code()` output to a file (file picker or fixed path)
11. **Load `.glicol`**: parse a `.glicol` file into the `Graph` structure and push to engine
12. *(Optional)* Proprietary JSON/RON graph format for preserving UI layout, node positions, metadata

### Phase 5 — Polish & extras

13. Node connection UI — visual way to wire node inputs (currently only via action)
14. Category-filtered node browser for adding nodes by type
15. BPM control from the TUI
16. Sample management (list loaded samples, load new ones at runtime)
17. Help overlay / keybinding cheat sheet

## Architecture Notes

```
main.rs              -> CLI args, launches App
app.rs               -> App struct: owns Engine, components, action loop, audio thread
tui.rs               -> Terminal setup, event loop (crossterm + tokio)
action.rs            -> Action enum (all user/system actions)
config.rs            -> JSON5 config + keybinding parsing
graph/mod.rs         -> Graph, Node (data model, AST export, to_glicol_code())
graph/node_types.rs  -> NodeRegistry, NodeTypeDefinition, ParameterType
components/
  mod.rs             -> Component trait
  home.rs            -> Home screen (placeholder "hello world")
  graph.rs           -> GraphComponent (node list, details, categories, editing)
  log_display.rs     -> Log panel at bottom of screen
```

- **Audio runs on a dedicated OS thread** (`thread::spawn` in `App::new`), engine is behind `Arc<Mutex<Engine>>`
- **Component trait** provides `draw`, `update`, `handle_events` — registered in `App.components` vec
- **GraphComponent is duplicated** in both `App.components` and `App.graph_component` — needs cleanup
- **Keybindings** are mode-scoped: `Home`, `Graph`, `GraphEditing` each have their own keymap in `config.json5`
