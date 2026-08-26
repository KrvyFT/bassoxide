use rustysynth::SoundFont;
use std::fs::File;
fn main() {
    let mut f = File::open("assets/Orchestra_HQ.sf2").unwrap();
    let sf = SoundFont::new(&mut f).unwrap();
    for p in sf.get_presets() {
        println!("{} - {}", p.get_patch_number(), p.get_name());
    }
}
