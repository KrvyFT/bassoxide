use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::sync::Arc;

fn main() {
    let mut sf2_file = File::open("assets/Orchestra_HQ.sf2").unwrap();
    let soundfont = Arc::new(SoundFont::new(&mut sf2_file).unwrap());
    let settings = SynthesizerSettings::new(44100);
    let mut synth = Synthesizer::new(&soundfont, &settings).unwrap();

    synth.process_midi_message(0, 0xB0, 0, 1); // Bank 1
    synth.process_midi_message(0, 0xC0, 36, 0); // Patch 36
    synth.reset();
    
    // Test what patch it is now? We can't easily read it, but we can assume it's 0.
}
