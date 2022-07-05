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
    let config: cpal::StreamConfig = input_device.default_input_config()?.into();
    println!("input config {:?}", config);

    let tone_time = Arc::new(Mutex::new(SystemTime::now()));
    let input_tone_time = tone_time.clone();

    let samples_per_ms = match config.sample_rate.0 {
        48000 => 48,
        32000 => 32,
        16000 => 16,
        _ => panic!(),
    };

    let input_data_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
        println!("Mic: {:?}", data.len());
        for (i, &sample) in data.iter().enumerate() {
            if sample > 0.1 {
                let index = i / config.channels as usize;
                let ms = index / samples_per_ms;
                {
                    let tt = input_tone_time.lock().unwrap();
                    println!("{:?}", tt.elapsed().unwrap() + Duration::from_millis(ms as u64));
                }

            }
        }
    };

    let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        println!("Speaker: {:?}", data.len());
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
        config
    );
    let input_stream = input_device.build_input_stream(&config, input_data_fn, err_fn)?;
    let output_stream = output_device.build_output_stream(&config, output_data_fn, err_fn)?;
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
