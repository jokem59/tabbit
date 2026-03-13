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
    pub assignments: Vec<(String, u8)>, // (StringName, Fret)
}

pub fn find_voicings(midi_pitches: &[u8]) -> Vec<Voicing> {
    if midi_pitches.is_empty() { return vec![]; }
    
    let string_pitches = [
        ("e", 64), ("B", 59), ("G", 55), ("D", 50), ("A", 45), ("E", 40)
    ];

    // 1. Find all possible (string, fret) for each pitch
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

    // 2. Cartesian product of all options
    use itertools::Itertools;
    let all_combos = options.into_iter().multi_cartesian_product();

    // 3. Filter
    all_combos.into_iter().filter_map(|combo| {
        // Must use different strings
        let mut used_strings = std::collections::HashSet::new();
        for (s, _) in &combo {
            if !used_strings.insert(s) { return None; }
        }

        // Fret stretch limit (max 4 frets difference, ignoring open strings)
        let non_zero_frets: Vec<u8> = combo.iter().map(|&(_, f)| f).filter(|&f| f > 0).collect();
        if !non_zero_frets.is_empty() {
            let min_f = *non_zero_frets.iter().min().unwrap();
            let max_f = *non_zero_frets.iter().max().unwrap();
            if max_f - min_f > 4 { return None; }
        }

        Some(Voicing { assignments: combo.into_iter().map(|(s, f)| (s, f)).collect() })
    }).collect()
}
