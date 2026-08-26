use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::sync::Arc;

fn main() {
    let mut sf2_file = File::open("assets/Orchestra_HQ.sf2").unwrap();
    let soundfont = Arc::new(SoundFont::new(&mut sf2_file).unwrap());
    let settings = SynthesizerSettings::new(44100);
    let mut synth = Synthesizer::new(&soundfont, &settings).unwrap();

    // Distorted guitar is Bank 0, Patch 30
    synth.process_midi_message(0, 0xB0, 0, 0); // Bank 0
    synth.process_midi_message(0, 0xC0, 30, 0); // Patch 30

    // Play note 60
    synth.process_midi_message(0, 0x90, 60, 100);

    let mut left = vec![0.0f32; 1000];
    let mut right = vec![0.0f32; 1000];
    synth.render(&mut left, &mut right);

    let sum: f32 = left.iter().map(|v| v.abs()).sum();
    println!("Sum left: {}", sum);
}
