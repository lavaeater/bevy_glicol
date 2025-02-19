use color_eyre::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample,
};
use crossterm::event::KeyEvent;
use glicol::Engine;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{
    action::Action,
    components::{
        graph::GraphComponent, home::Home, log_display::LogDisplay, Component,
    },
    config::Config,
    tui::{Event, Tui},
};

const SPECIAL: &str = include_str!("../.config/synth.txt");
const SAMPLES: &str = include_str!("../.config/sample-list.json");
const BLOCK_SIZE: usize = 128;

pub struct App {
    config: Config,
    frame_rate: f64,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    engine: Arc<Mutex<Engine<BLOCK_SIZE>>>,
    stream: Option<cpal::Stream>,
    graph_component: GraphComponent<BLOCK_SIZE>,
    log_display: LogDisplay,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
}

impl App {
    pub fn new(frame_rate: f64) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut engine = Engine::<BLOCK_SIZE>::new();
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

        engine
            .update_with_code(r#"out: saw 440.0 >> mul 0.1"#)?;
        let engine = Arc::new(Mutex::new(engine));

        let mut graph_component = GraphComponent::new();

        if let Ok(engine) = engine.lock() {
            graph_component.update_node_count(engine.context.graph.node_count());
        }
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(device) => device,
            None => {
                let err_msg = "No output device found";
                tracing::error!("{err_msg}");
                return Err(color_eyre::eyre::eyre!(err_msg));
            }
        };

        let config = device.default_output_config()?;

        let engine_clone = engine.clone();
        match config.sample_format() {
            cpal::SampleFormat::F32 => {
                thread::spawn(move || run_audio::<f32>(&device, &config.into(), engine_clone))
            }
            sample_format => {
                panic!("Unsupported sample format '{sample_format}'")
            }
        };

        Ok(Self {
            frame_rate,
            components: vec![
                Box::new(Home::new()),
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
                            self.graph_component
                                .update_node_count(engine.context.graph.node_count());
                        }
                    }
                }
                Action::SpecialAudio => {
                    if let Ok(mut engine) = self.engine.lock() {
                        match engine.update_with_code(SPECIAL) {
                            Ok(_) => self
                                .graph_component
                                .update_node_count(engine.context.graph.node_count()),
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
        // let host = cpal::default_host();
        // let device = match host.default_output_device() {
        //     Some(device) => device,
        //     None => {
        //         let err_msg = "No output device found";
        //         tracing::error!("{err_msg}");
        //         self.log_display.add_error(err_msg.to_string());
        //         return Err(color_eyre::eyre::eyre!(err_msg));
        //     }
        // };
        // 
        // let config = device.default_output_config()?;
        // 
        // let engine_clone = self.engine.clone();
        // match config.sample_format() {
        //     cpal::SampleFormat::F32 => {
        //         thread::spawn(move || run_audio::<f32>(&device, &config.into(), engine_clone))
        //     }
        //     sample_format => {
        //         panic!("Unsupported sample format '{sample_format}'")
        //     }
        // };
        Ok(())
    }
}

fn run_audio<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    engine: Arc<Mutex<Engine<BLOCK_SIZE>>>,
) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let sr = config.sample_rate.0 as usize;
    let channels = 2_usize; //config.channels as usize;

    if let Ok(mut engine) = engine.lock() {
        engine.set_sr(sr);
        engine.livecoding = false;
    }

    let engine_clone = engine.clone();

    let mut prev_block: [glicol_synth::Buffer<BLOCK_SIZE>; 2] = [glicol_synth::Buffer::SILENT; 2];

    let ptr = prev_block.as_mut_ptr();
    let prev_block_ptr = Arc::new(AtomicPtr::<glicol_synth::Buffer<BLOCK_SIZE>>::new(ptr));
    let prev_block_len = Arc::new(AtomicUsize::new(prev_block.len()));

    let mut prev_block_pos: usize = BLOCK_SIZE;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let block_step = data.len() / channels;

            let mut write_samples =
                |block: &[glicol_synth::Buffer<BLOCK_SIZE>], sample_i: usize, i: usize| {
                    for chan in 0..channels {
                        let value: T = T::from_sample(block[chan][i]);
                        data[sample_i * channels + chan] = value;
                    }
                };

            let ptr = prev_block_ptr.load(Ordering::Acquire);
            let len = prev_block_len.load(Ordering::Acquire);
            let prev_block: &mut [glicol_synth::Buffer<BLOCK_SIZE>] =
                unsafe { std::slice::from_raw_parts_mut(ptr, len) };

            let mut writes = 0;

            for i in prev_block_pos..BLOCK_SIZE {
                write_samples(prev_block, writes, i);
                writes += 1;
            }

            prev_block_pos = BLOCK_SIZE;
            while writes < block_step {
                let mut e = engine_clone.lock().unwrap();
                let block = e.next_block(vec![]);

                if writes + BLOCK_SIZE <= block_step {
                    for i in 0..BLOCK_SIZE {
                        write_samples(block, writes, i);
                        writes += 1;
                    }
                } else {
                    let e = block_step - writes;
                    for i in 0..e {
                        write_samples(block, writes, i);
                        writes += 1;
                    }
                    for (buffer, block) in prev_block.iter_mut().zip(block.iter()) {
                        buffer.copy_from_slice(block);
                    }
                    prev_block_pos = e;
                    break;
                }
            }
        },
        |err| error!("an error occurred on stream: {err}"),
        None,
    )?;
    stream.play()?;
    loop {
        thread::park() // wait forever
    }
}
