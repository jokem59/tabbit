pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub fn get_note_name(midi_pitch: u8) -> String {
    NOTE_NAMES[(midi_pitch % 12) as usize].to_string()
}

pub fn analyze_chord(notes: &[u8], root_hint: Option<&str>) -> String {
    if notes.is_empty() { return String::new(); }
    
    let mut unique_notes: Vec<u8> = notes.iter().cloned().collect();
    unique_notes.sort();
    unique_notes.dedup_by(|a, b| *a % 12 == *b % 12);

    let root_pitch_class = if let Some(hint) = root_hint {
        NOTE_NAMES.iter().position(|&n| n.to_uppercase() == hint.to_uppercase())
            .map(|p| p as u8)
            .unwrap_or(unique_notes[0] % 12)
    } else {
        unique_notes[0] % 12
    };

    let root_name = if let Some(hint) = root_hint {
        hint.to_uppercase()
    } else {
        get_note_name(unique_notes[0])
    };

    if unique_notes.len() == 1 {
        return root_name;
    }

    let intervals: Vec<u8> = unique_notes.iter()
        .map(|&n| (n as i16 - root_pitch_class as i16).rem_euclid(12) as u8)
        .collect();

    let interval_names = |i: u8| match i {
        0 => "1", 1 => "b2", 2 => "2", 3 => "b3", 4 => "3", 5 => "4",
        6 => "b5", 7 => "5", 8 => "b6", 9 => "6", 10 => "b7", 11 => "7",
        _ => "?",
    };

    let readable_ints: String = intervals.iter()
        .map(|&i| interval_names(i))
        .collect::<Vec<_>>()
        .join("-");

    let mut quality = "";
    let int_set: std::collections::HashSet<u8> = intervals.into_iter().collect();
    
    if int_set.contains(&0) && int_set.contains(&4) && int_set.contains(&7) {
        quality = "Maj";
    } else if int_set.contains(&0) && int_set.contains(&3) && int_set.contains(&7) {
        quality = "Min";
    } else if int_set.contains(&0) && int_set.contains(&7) && unique_notes.len() == 2 {
        quality = "Pwr";
    }

    format!("{}{} [{}]", root_name, quality, readable_ints)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Voicing {
    pub assignments: Vec<(String, u8)>,
}

pub fn find_voicings(midi_pitches: &[u8]) -> Vec<Voicing> {
    if midi_pitches.is_empty() { return vec![]; }
    
    let string_pitches = [
        ("e", 64), ("B", 59), ("G", 55), ("D", 50), ("A", 45), ("E", 40)
    ];

    let mut options = Vec::new();
    for &pitch in midi_pitches {
        let mut pitch_options = Vec::new();
        for (s_name, s_pitch) in string_pitches {
            if pitch >= s_pitch {
                let fret = pitch - s_pitch;
                if fret <= 15 {
                    pitch_options.push((s_name.to_string(), fret));
                }
            }
        }
        options.push(pitch_options);
    }

    use itertools::Itertools;
    let all_combos = options.into_iter().multi_cartesian_product();

    all_combos.into_iter().filter_map(|combo| {
        let mut used_strings = std::collections::HashSet::new();
        for (s, _) in &combo {
            if !used_strings.insert(s) { return None; }
        }

        let non_zero_frets: Vec<u8> = combo.iter().map(|&(_, f)| f).filter(|&f| f > 0).collect();
        if !non_zero_frets.is_empty() {
            let min_f = *non_zero_frets.iter().min().unwrap();
            let max_f = *non_zero_frets.iter().max().unwrap();
            if max_f - min_f > 4 { return None; }
        }

        Some(Voicing { assignments: combo.into_iter().map(|(s, f)| (s, f)).collect() })
    }).collect()
}

pub struct MidiPlayer {
    #[cfg(feature = "midi")]
    conn: Option<midir::MidiOutputConnection>,
    pub port_name: String,
}

impl MidiPlayer {
    pub fn new() -> Self {
        #[cfg(feature = "midi")]
        {
            let midi_out = midir::MidiOutput::new("Tabbit Player").ok();
            let mut port_name = "None".to_string();
            let conn = midi_out.and_then(|out| {
                let ports = out.ports();
                if ports.is_empty() {
                    port_name = "NO MIDI OUT FOUND (Install FluidSynth)".to_string();
                    None
                } else {
                    let target_port = ports.iter().find(|p| {
                        let name = out.port_name(p).unwrap_or_default().to_lowercase();
                        name.contains("fluid") || name.contains("timidity") || name.contains("synth")
                    }).or(ports.first());

                    if let Some(port) = target_port {
                        port_name = out.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
                        out.connect(port, "tabbit-out").ok()
                    } else {
                        None
                    }
                }
            });

            Self { conn, port_name }
        }

        #[cfg(not(feature = "midi"))]
        {
            Self { port_name: "MIDI Disabled (Compile with --features midi)".to_string() }
        }
    }

    pub fn play_notes(&mut self, midi_pitches: &[u8], duration_ms: u64) {
        #[cfg(feature = "midi")]
        if let Some(ref mut conn) = self.conn {
            for &pitch in midi_pitches {
                let _ = conn.send(&[0x90, pitch, 0x64]);
            }
            std::thread::sleep(std::time::Duration::from_millis(duration_ms));
            for &pitch in midi_pitches {
                let _ = conn.send(&[0x80, pitch, 0x64]);
            }
        }

        #[cfg(not(feature = "midi"))]
        {
            let _ = (midi_pitches, duration_ms);
        }
    }
}
