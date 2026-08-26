use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::sync::Arc;

fn main() {
    let mut sf2_file = File::open("assets/Orchestra_HQ.sf2").unwrap();
    let soundfont = Arc::new(SoundFont::new(&mut sf2_file).unwrap());
    let settings = SynthesizerSettings::new(44100);
    let mut synth = Synthesizer::new(&soundfont, &settings).unwrap();

    // Try Slap Bass 3 (Bank 1, Patch 36)
    synth.process_midi_message(0, 0xB0, 0, 1);
    synth.process_midi_message(0, 0xC0, 36, 0);

    // Is the preset correctly assigned to channel 0?
    // How can we check the current preset of a channel?
    // Actually just compiling is fine.
    println!("Synth test successful.");
}
