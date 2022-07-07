//! Feeds back the input stream directly into the output stream.
//!
//! Assumes that the input and output devices can use the same stream configuration and that they
//! support the f32 sample format.
//!
//! Uses a delay of `LATENCY_MS` milliseconds in case the default input and output streams are not
//! precisely synchronised.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::{Duration, SystemTime};
use std::sync::{Arc, Mutex};

fn main() -> anyhow::Result<()> {
    let host = cpal::default_host();

    // Find devices.
    let input_device = host.default_input_device().unwrap();
    let output_device = host.default_output_device().unwrap();

    println!("Using input device: \"{}\"", input_device.name()?);
    println!("Using output device: \"{}\"", output_device.name()?);

    // We'll try and use the same configuration between streams to keep it simple.
    // let input_config = input_device.default_input_config()?;
    // let input_config: cpal::StreamConfig = input_config.into();
    // println!("input config {:?}", input_config);
    // let output_config: cpal::StreamConfig = output_device.default_output_config()?.into();
    // println!("output config {:?}", output_config);

    let input_config: cpal::StreamConfig = cpal::StreamConfig { channels: 2, sample_rate: cpal::SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(128) };
    let output_config: cpal::StreamConfig = cpal::StreamConfig { channels: 2, sample_rate: cpal::SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(128) };

    // let input_config: cpal::StreamConfig = cpal::StreamConfig { channels: 2, sample_rate: cpal::SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(480) };
    // let output_config: cpal::StreamConfig = cpal::StreamConfig { channels: 2, sample_rate: cpal::SampleRate(48000), buffer_size: cpal::BufferSize::Fixed(480) };

    assert!(input_config == output_config);

    let tone_time = Arc::new(Mutex::new(SystemTime::now()));
    let input_tone_time = tone_time.clone();

    let input_samples_per_ms = match input_config.sample_rate.0 {
        48000 => 48,
        32000 => 32,
        16000 => 16,
        _ => panic!(),
    };

    let mut input_log_once = true;
    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        if input_log_once {
            println!("Mic frames (samples per channel): {:?}", data.len() / input_config.channels as usize);
            input_log_once = false;
        }
        
        let mut log_delay_once = true;
        for (i, &sample) in data.iter().enumerate() {
            if sample > 0.3 {
                let index = i / input_config.channels as usize;
                let ms = index / input_samples_per_ms;
                {
                    if log_delay_once {
                        let tt = input_tone_time.lock().unwrap();
                        println!("{:?}", tt.elapsed().unwrap() + Duration::from_millis(ms as u64));
                        log_delay_once = false;
                    }
                }

            }
        }
    };

    let mut output_log_once = true;
    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        if output_log_once && data.len() <= 279 {
            println!("Speaker frames (samples per channel): {:?}", data.len() / input_config.channels as usize);
            output_log_once = false;
        }
        let mut tt = tone_time.lock().unwrap();
        if tt.elapsed().unwrap() > Duration::from_secs(1) {
            data[0] = 1.;
            *tt = SystemTime::now();
        } else {
            data[0] = 0.;
        }
        for sample in &mut data[1..] {
            *sample = 0.
        }
    };

    // Build streams.
    println!(
        "Attempting to build both streams with f32 samples and `{:?}`.",
        input_config
    );
    let input_stream = input_device.build_input_stream(&input_config, input_data_fn, err_fn)?;
    let output_stream = output_device.build_output_stream(&output_config, output_data_fn, err_fn)?;
    println!("Successfully built streams.");

    input_stream.play()?;
    output_stream.play()?;

    // Run for 3 seconds before closing.
    println!("Playing for 3 seconds... ");
    std::thread::sleep(std::time::Duration::from_secs(600));
    drop(input_stream);
    drop(output_stream);
    println!("Done!");
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}
