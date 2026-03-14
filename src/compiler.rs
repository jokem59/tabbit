use crate::parser::{TabbitCommand, NoteEvent};
use crate::theory::analyze_chord;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Song {
    pub time_signature: (u32, u32),
    pub bpm: u32,
    pub measures: HashMap<u32, HashMap<String, BeatData>>,
    pub section_headers: HashMap<u32, String>,
    pub max_measure: u32,
}

#[derive(Debug, Clone)]
pub struct BeatData {
    pub notes: Vec<NoteEvent>,
    pub theory: String,
    pub cue: String,
}

pub struct Compiler {
    macros: HashMap<String, Vec<TabbitCommand>>,
    measure_offset: u32,
    string_pitches: HashMap<String, u8>,
    current_section: Option<String>,
}

impl Compiler {
    pub fn new() -> Self {
        let mut string_pitches = HashMap::new();
        string_pitches.insert("e".to_string(), 64);
        string_pitches.insert("B".to_string(), 59);
        string_pitches.insert("G".to_string(), 55);
        string_pitches.insert("D".to_string(), 50);
        string_pitches.insert("A".to_string(), 45);
        string_pitches.insert("E".to_string(), 40);

        Self {
            macros: HashMap::new(),
            measure_offset: 0,
            string_pitches,
            current_section: None,
        }
    }

    pub fn compile(&mut self, commands: &[TabbitCommand]) -> Song {
        let mut song = Song {
            time_signature: (4, 4),
            bpm: 120,
            measures: HashMap::new(),
            section_headers: HashMap::new(),
            max_measure: 0,
        };
        self.process_commands(commands, &mut song);
        song
    }

    fn process_commands(&mut self, commands: &[TabbitCommand], song: &mut Song) {
        for cmd in commands {
            match cmd {
                TabbitCommand::Bpm(val) => {
                    song.bpm = *val;
                }
                TabbitCommand::SectionHeader(title) => {
                    self.current_section = Some(title.clone());
                }
                TabbitCommand::TimeSignature(num, den) => {
                    song.time_signature = (*num, *den);
                }
                TabbitCommand::MeasureEvent { measure, beat, notes, root_hint, cue } => {
                    let m_num = *measure + self.measure_offset;
                    song.max_measure = song.max_measure.max(m_num);
                    
                    if let Some(section) = self.current_section.take() {
                        song.section_headers.entry(m_num).or_insert(section);
                    }
                    
                    let midi_pitches: Vec<u8> = notes.iter().filter_map(|n| {
                        let base = self.string_pitches.get(&n.string)?;
                        let fret = n.fret.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<u8>().ok()?;
                        Some(base + fret)
                    }).collect();

                    let theory = analyze_chord(&midi_pitches, root_hint.as_deref());
                    
                    let measure_map = song.measures.entry(m_num).or_insert_with(HashMap::new);
                    measure_map.insert(beat.to_string(), BeatData {
                        notes: notes.clone(),
                        theory,
                        cue: cue.clone().unwrap_or_default(),
                    });
                }
                TabbitCommand::Repeat { count, commands: repeat_cmds } => {
                    for i in 0..*count {
                        self.process_commands(repeat_cmds, song);
                        if i < *count - 1 {
                            self.measure_offset = song.max_measure;
                        }
                    }
                }
                TabbitCommand::MacroDefine { name, commands: macro_cmds } => {
                    self.macros.insert(name.clone(), macro_cmds.clone());
                }
                TabbitCommand::MacroUsage(name) => {
                    if let Some(macro_cmds) = self.macros.get(name).cloned() {
                        self.process_commands(&macro_cmds, song);
                    }
                }
                TabbitCommand::RootHint(_) => {
                    // These are normally consumed by the parser's combination logic,
                    // but we include the arm to satisfy the compiler.
                }
            }
        }
    }
}
