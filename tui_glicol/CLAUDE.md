# TUI Glicol — CLAUDE.md

## Project Overview

A terminal user interface for the [Glicol](https://glicol.org/) music engine, built in Rust with `ratatui` and `cpal`. Glicol uses a node-based DSL to connect oscillators, samples, beats, effects, and math operations into audio graphs. This TUI makes that graph live-editable without writing raw Glicol code.

## Architecture

```
src/
  main.rs              -> CLI args, launches App
  app.rs               -> App struct: owns Engine, components, action loop, audio thread
  tui.rs               -> Terminal setup, event loop (crossterm + tokio)
  action.rs            -> Action enum (all user/system actions)
  config.rs            -> JSON5 config + keybinding parsing
  graph/mod.rs         -> Graph, Node (data model, AST export, to_glicol_code())
  graph/node_types.rs  -> NodeRegistry, NodeTypeDefinition, ParameterType
  components/
    mod.rs             -> Component trait
    home.rs            -> Home screen
    graph.rs           -> GraphComponent (node list, details, categories, editing)
    log_display.rs     -> Log panel at bottom of screen
```

**Key design points:**
- Audio runs on a dedicated OS thread (`thread::spawn` in `App::new`), engine behind `Arc<Mutex<Engine>>`
- `Component` trait: provides `draw`, `update`, `handle_events` — registered in `App.components` vec
- Keybindings are mode-scoped: `Home`, `Graph`, `GraphEditing` each have their own keymap in `config.json5`
- Graph edits automatically call `Graph::to_glicol_code()` → `engine.update_with_code()` (live audio sync)
- `Graph::to_glicol_code()` serializes the node graph into Glicol DSL with chain flattening (e.g. `sin 440 >> mul 0.3`)

## Current Status

**Phase 2 complete.** Graph-Engine round-trip is working.

### What works
- Audio engine: Glicol + cpal, plays on startup
- Sample loading from `.config/sample-list.json`
- Keybinding config: JSON5, mode-aware (`Home`, `Graph`, `GraphEditing`)
- Actions: `PlayAudio`, `StopAudio`, `UpdateAudioCode`, `SpecialAudio`, `SwitchMode`
- Graph data model: `Graph`, `Node`, `NodeRegistry`, `NodeTypeDefinition` — add/remove/connect, param validation, `to_glicol_ast()`, `to_glicol_code()`
- Graph-Engine sync: graph edits regenerate code and call engine
- Graph component UI: category tabs, node list, node detail panel, error bar
- Graph actions: `j`/`k` navigation, `h`/`l` category browsing, add/remove nodes, param nav (`Tab`/`Shift-Tab`)
- Log display at bottom

### Known issues / not yet done
- **Mode-conditional rendering broken**: `render()` always draws all components; `GraphComponent` is drawn twice (once via `self.components`, once directly). Fix: draw `Home` only in `Home` mode, `GraphComponent` only in `Graph`/`GraphEditing`.
- **`GraphComponent` is duplicated** in both `App.components` and `App.graph_component` — needs cleanup.
- **No text input for params**: `GraphEditParam` uses hardcoded example values; needs actual text input widget.
- **No reverse sync**: no path from `.glicol` file → `Graph`.
- **No file I/O**: no save/load for `.glicol` files or graph format.

## Roadmap

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | Pending | Mode-conditional rendering, fix double-draw, default seed patch |
| 2 | ✅ Done | Graph→Engine round-trip (`to_glicol_code()` wired to engine) |
| 3 | Next | Parameter editing: text input widget, all `ParameterType` variants |
| 4 | Future | File I/O: save/load `.glicol`, optional JSON/RON graph format |
| 5 | Future | Node connection UI, category browser, BPM control, sample management, help overlay |

## Glicol DSL Notes

Glicol chains are written as: `out: sin 440 >> mul 0.3`
- Left of `:` is the output node name
- `>>` chains nodes
- References between nodes use `~name` syntax
- Patterns use Glicol's mini-notation

## Key Dependencies

- `ratatui` — TUI rendering
- `cpal` — cross-platform audio output
- `glicol` (or glicol engine crate) — audio DSP engine
- `tokio` — async runtime for event loop
- `crossterm` — terminal backend
- `json5` — config/keybindings format
