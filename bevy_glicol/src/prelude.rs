use bevy::prelude::*;
use cpal::{traits::*, FromSample, SizedSample};
use parking_lot::Mutex;
use std::{
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

const BLOCK_SIZE: usize = 128;

#[derive(Resource)]
pub struct GlicolEngine {
    pub engine: Arc<Mutex<glicol::Engine<BLOCK_SIZE>>>,
}

impl GlicolEngine {
    pub fn new() -> Self {
        let engine = Arc::new(Mutex::new(glicol::Engine::<BLOCK_SIZE>::new()));
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("No default output device found");
        let config = device.default_output_config().unwrap();
        info!("Default output config: {:?}", config);

        let engine_clone = engine.clone();

        thread::spawn(move || match config.sample_format() {
            cpal::SampleFormat::F32 => run_audio::<f32>(&device, &config.into(), engine_clone),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        });

        Self { engine }
    }

    pub fn update_with_code(&self, code: &str) {
        let mut engine = self.engine.lock();
        if let Err(e) = engine.update_with_code(code) {
            error!("Failed to update Glicol code: {}", e);
        }
    }
}

fn run_audio<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    engine: Arc<Mutex<glicol::Engine<BLOCK_SIZE>>>,
) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let sr = config.sample_rate.0 as usize;
    let channels = 2_usize; //config.channels as usize;

    engine.lock().set_sr(sr);
    engine.lock().livecoding = false;

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
                let mut e = engine_clone.lock();
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

// Glicol bevy plugin
pub struct GlicolPlugin;

impl Plugin for GlicolPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GlicolEngine::new());
    }
}
