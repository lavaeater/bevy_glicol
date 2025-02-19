use color_eyre::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossterm::event::KeyEvent;
use glicol::Engine;
use hound;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{
    action::Action,
    components::{
        fps::FpsCounter, graph::GraphComponent, home::Home, log_display::LogDisplay, Component,
    },
    config::Config,
    tui::{Event, Tui},
};

const SPECIAL: &str = include_str!("../.config/synth.txt");
const SAMPLES: &str = include_str!("../.config/sample-list.json");

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    engine: Arc<Mutex<Engine<512>>>,
    stream: Option<cpal::Stream>,
    graph_component: GraphComponent<512>,
    log_display: LogDisplay,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}

impl App {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut engine = Engine::<512>::new();
        // match fs::read_to_string("../.config/sample-list.json") {
        // Ok(json_content) => {
        if let Ok(sample_map) = serde_json::from_str::<HashMap<String, String>>(SAMPLES) {
            for (name, path) in sample_map {
                match hound::WavReader::open(&path) {
                    Ok(mut reader) => {
                        let spec = reader.spec();
                        let samples: Vec<f32> = match spec.sample_format {
                            hound::SampleFormat::Float => {
                                reader.samples::<f32>().filter_map(Result::ok).collect()
                            }
                            hound::SampleFormat::Int => {
                                if spec.bits_per_sample == 16 {
                                    reader
                                        .samples::<i16>()
                                        .filter_map(Result::ok)
                                        .map(|s| s as f32 / 32768.0)
                                        .collect()
                                } else {
                                    error!("Unsupported bits per sample: {}", spec.bits_per_sample);
                                    continue;
                                }
                            }
                        };

                        // Convert samples to static lifetime - this is safe because the Engine keeps them for its entire lifetime
                        let samples_static = Box::leak(samples.into_boxed_slice());
                        engine.add_sample(
                            &name,
                            samples_static,
                            spec.channels as usize,
                            spec.sample_rate as usize,
                        );
                        info!("Loaded sample: {}", name);
                    }
                    Err(e) => error!("Failed to read WAV file {}: {}", path, e),
                }
            }
        }
        //     }
        //     Err(err) => {
        //         error!("{}", err);
        //         log_lines.push(format!("{}", err));
        //     }
        // }

        engine
            .update_with_code(r#"out: saw 440.0 >> mul 0.1"#)
            .unwrap();
        let engine = Arc::new(Mutex::new(engine));

        let mut graph_component = GraphComponent::new();
        graph_component.set_engine(engine.clone());
        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![
                Box::new(Home::new()),
                Box::new(FpsCounter::default()),
                Box::new(graph_component.clone()),
            ],
            log_display: LogDisplay::default(),
            should_quit: false,
            should_suspend: false,
            config: Config::new()?,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            engine,
            stream: None,
            graph_component,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        let Some(keymap) = self.config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            let action_for_components = action.clone();
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::PlayAudio => self.setup_audio()?,
                Action::StopAudio => {
                    if let Some(stream) = self.stream.take() {
                        drop(stream);
                    }
                }
                Action::UpdateAudioCode(code) => {
                    if let Ok(mut engine) = self.engine.lock() {
                        if engine.update_with_code(&code).is_ok() {
                            self.graph_component.set_engine(self.engine.clone());
                        }
                    }
                }
                Action::SpecialAudio => {
                    if let Ok(mut engine) = self.engine.lock() {
                        match engine.update_with_code(&SPECIAL) {
                            Ok(_) => self.graph_component.set_engine(self.engine.clone()),
                            Err(e) => {
                                let err_msg = format!("Failed to update SPECIAL Glicol code: {e}");
                                error!("{err_msg}");
                                self.log_display.add_error(err_msg);
                            }
                        }
                    }
                }
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(new_action) = component.update(action_for_components.clone())? {
                    self.action_tx.send(new_action)?
                };
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            let area = frame.area();
            let graph_area = Rect::new(0, 0, area.width, area.height - 6);
            let log_area = Rect::new(0, area.height - 6, area.width, 6);

            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, graph_area) {
                    let err_msg = format!("Failed to draw: {:?}", err);
                    self.log_display.add_error(err_msg.clone());
                    let _ = self.action_tx.send(Action::Error(err_msg));
                }
            }

            if let Err(err) = self.graph_component.draw(frame, graph_area) {
                let err_msg = format!("Failed to draw graph: {:?}", err);
                self.log_display.add_error(err_msg.clone());
                let _ = self.action_tx.send(Action::Error(err_msg));
            }

            if let Err(err) = self.log_display.draw(frame, log_area) {
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Failed to draw logs: {:?}", err)));
            }
        })?;
        Ok(())
    }

    fn setup_audio(&mut self) -> Result<()> {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(device) => device,
            None => {
                let err_msg = "No output device found";
                tracing::error!("{err_msg}");
                self.log_display.add_error(err_msg.to_string());
                return Err(color_eyre::eyre::eyre!(err_msg));
            }
        };

        let config = device.default_output_config()?.config();

        let engine = self.engine.clone();
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut engine) = engine.lock() {
                    // Process the next block
                    let empty_buffer = [0.0f32; 512];
                    let input_buffers = vec![&empty_buffer[..]; 8];
                    let output = engine.next_block(input_buffers);
                    // Copy the output data
                    for (i, sample) in data.iter_mut().enumerate() {
                        *sample = output[0][i % 512];
                    }
                }
            },
            |err| {
                tracing::error!("Audio stream error: {}", err);
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }
}
