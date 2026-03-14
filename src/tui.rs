use crate::compiler::Song;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// Dark Plus Theme Colors
const BG_DARK: Color = Color::Rgb(30, 30, 30);
const FG_LIGHT: Color = Color::Rgb(212, 212, 212);
const CHORD_BLUE: Color = Color::Rgb(156, 220, 254);
const CUE_MAGENTA: Color = Color::Rgb(197, 134, 192);
const FRET_ORANGE: Color = Color::Rgb(206, 145, 120);
const BEAT_GRAY: Color = Color::Rgb(128, 128, 128);
const HIGHLIGHT_BG: Color = Color::Rgb(45, 45, 45);
const BORDER_GRAY: Color = Color::Rgb(64, 64, 64);
const SECTION_CYAN: Color = Color::Rgb(78, 201, 176);

use std::sync::{Arc, RwLock, atomic::{AtomicBool, AtomicU32, Ordering}};

pub struct TuiApp {
    pub song: Arc<RwLock<Song>>,
    pub cursor_measure: Arc<AtomicU32>,
    pub cursor_beat_scaled: Arc<AtomicU32>, // Store as (beat * 100)
    pub cursor_string: usize,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
    pub is_playing: Arc<AtomicBool>,
    pub midi_port: String,
}

impl TuiApp {
    pub fn new(song: Song) -> Self {
        let player = crate::theory::MidiPlayer::new();
        Self {
            song: Arc::new(RwLock::new(song)),
            cursor_measure: Arc::new(AtomicU32::new(1)),
            cursor_beat_scaled: Arc::new(AtomicU32::new(100)),
            cursor_string: 0,
            vertical_scroll: 0,
            horizontal_scroll: 0,
            is_playing: Arc::new(AtomicBool::new(false)),
            midi_port: player.port_name,
        }
    }

    fn get_cursor_measure(&self) -> u32 {
        self.cursor_measure.load(Ordering::SeqCst)
    }

    fn get_cursor_beat(&self) -> f32 {
        self.cursor_beat_scaled.load(Ordering::SeqCst) as f32 / 100.0
    }

    fn set_cursor_measure(&self, m: u32) {
        self.cursor_measure.store(m, Ordering::SeqCst);
    }

    fn set_cursor_beat(&self, b: f32) {
        self.cursor_beat_scaled.store((b * 100.0) as u32, Ordering::SeqCst);
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_loop(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        res
    }

    fn calculate_measure_width(&self, m_num: u32) -> usize {
        let mut width = 2; // Start padding "--"
        let mut step = 1.0;
        
        let song_guard = self.song.read().unwrap();
        let m_data = match song_guard.measures.get(&m_num) {
            Some(d) if !d.is_empty() => d,
            _ => return 12,
        };

        for b_str in m_data.keys() {
            let b: f32 = b_str.parse().unwrap_or(1.0);
            let frac = (b % 1.0).abs();
            if frac > 0.001 {
                let mut s = 1.0;
                while (frac % s).abs() > 0.001 && s > 0.03125 { s /= 2.0; }
                if s < step { step = s; }
            }
        }

        let num_slots = (song_guard.time_signature.0 as f32 / step).round() as u32;
        let beat_sequence: Vec<f32> = (0..num_slots).map(|i| i as f32 * step + 1.0).collect();

        for b in beat_sequence {
            let b_str = b.to_string();
            let label = if b.fract() == 0.0 { b.to_string() } 
                        else if (b.fract() - 0.5).abs() < 0.001 { "&".to_string() }
                        else if (b.fract() - 0.25).abs() < 0.001 { "e".to_string() }
                        else if (b.fract() - 0.75).abs() < 0.001 { "a".to_string() }
                        else { ".".to_string() };

            let mut col_w = label.len().max(3);
            if let Some(bd) = m_data.get(&b_str) {
                for note in &bd.notes { col_w = col_w.max(note.fret.len()); }
            }
            width += col_w;
        }
        width.max(10)
    }

    fn get_measure_groups(&self, view_w: usize) -> Vec<(u32, u32)> {
        let song = self.song.read().unwrap();
        let mut groups = Vec::new();
        let mut current_m = 1;
        
        while current_m <= song.max_measure {
            let mut end_m = current_m;
            let mut current_w = 4; // "e  |"
            
            for m in current_m..=(current_m + 3).min(song.max_measure) {
                // ALWAYS START A NEW LINE FOR A NEW SECTION
                if m > current_m && song.section_headers.contains_key(&m) {
                    break;
                }

                let m_w = self.calculate_measure_width(m);
                // Ensure at least one measure is included even if it overflows
                if m > current_m && current_w + m_w + 1 > view_w {
                    break;
                }
                current_w += m_w + 1;
                end_m = m;
            }
            
            // Check if this range has any notes or the cursor or headers
            let mut has_notes = false;
            let cursor_m = self.get_cursor_measure();
            for m in current_m..=end_m {
                if let Some(m_data) = song.measures.get(&m) {
                    if !m_data.is_empty() { has_notes = true; break; }
                }
                if song.section_headers.contains_key(&m) { has_notes = true; break; }
            }
            let cursor_in = cursor_m >= current_m && cursor_m <= end_m;
            
            if has_notes || cursor_in {
                groups.push((current_m, end_m));
            }
            current_m = end_m + 1;
        }
        groups
    }

    fn run_loop<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()> {
        loop {
            let mut view_size = (0, 0);
            terminal.draw(|f| {
                self.ui(f);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
                    .split(f.size());
                view_size = (chunks[1].width as usize, chunks[1].height as usize);
            })?;

            // Auto-sync viewport if playing
            if self.is_playing.load(Ordering::SeqCst) {
                self.sync_viewport(view_size.0, view_size.1);
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') => self.move_cursor_beat(-0.25, view_size.0, view_size.1),
                        KeyCode::Char('l') => self.move_cursor_beat(0.25, view_size.0, view_size.1),
                        KeyCode::Char('j') => self.move_cursor_row(1, view_size.0, view_size.1),
                        KeyCode::Char('k') => self.move_cursor_row(-1, view_size.0, view_size.1),
                        KeyCode::Down => self.move_cursor_string(1, view_size.0, view_size.1),
                        KeyCode::Up => self.move_cursor_string(-1, view_size.0, view_size.1),
                        KeyCode::Char('w') => self.move_cursor_measure(1, view_size.0, view_size.1),
                        KeyCode::Char('b') => self.move_cursor_measure(-1, view_size.0, view_size.1),
                        KeyCode::Char('g') => {
                            self.set_cursor_measure(1);
                            self.set_cursor_beat(1.0);
                            self.cursor_string = 0;
                            self.vertical_scroll = 0;
                            self.horizontal_scroll = 0;
                        }
                        KeyCode::Char('G') => {
                            let max_m = self.song.read().unwrap().max_measure;
                            self.set_cursor_measure(max_m);
                            self.set_cursor_beat(1.0);
                            self.cursor_string = 5;
                            let (all_lines, _) = self.render_tab_lines_full(true, view_size.0, view_size.1);
                            if all_lines.len() > view_size.1 {
                                self.vertical_scroll = all_lines.len() - view_size.1;
                            }
                            self.sync_viewport(view_size.0, view_size.1);
                        }
                        KeyCode::Char('v') => self.cycle_voicing(),
                        KeyCode::Char('y') => self.yank_to_clipboard(),
                        KeyCode::Char(' ') => {
                            if self.is_playing.load(Ordering::SeqCst) {
                                self.is_playing.store(false, Ordering::SeqCst);
                            } else {
                                self.is_playing.store(true, Ordering::SeqCst);
                                self.start_playback();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn yank_to_clipboard(&self) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let (all_lines, _) = self.render_tab_lines_full(false, 999, 999);
            let ascii = all_lines.into_iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
            let _ = cb.set_text(ascii);
        }
    }

    fn move_cursor_measure(&mut self, delta: i32, view_w: usize, view_h: usize) {
        let current_m = self.get_cursor_measure();
        let next_m = {
            let song = self.song.read().unwrap();
            let mut defined_measures: Vec<_> = song.measures.keys().cloned().collect();
            defined_measures.sort();
            if defined_measures.is_empty() { return; }
            if delta > 0 { defined_measures.iter().find(|&&m| m > current_m).cloned() } 
            else { defined_measures.iter().rev().find(|&&m| m < current_m).cloned() }
        };
        if let Some(m) = next_m {
            self.set_cursor_measure(m);
            self.set_cursor_beat(1.0);
            self.sync_viewport(view_w, view_h);
        }
    }

    fn move_cursor_row(&mut self, delta: i32, view_w: usize, view_h: usize) {
        let groups = self.get_measure_groups(view_w);
        let cursor_m = self.get_cursor_measure();
        let current_group_idx = groups.iter().position(|&(s, e)| cursor_m >= s && cursor_m <= e).unwrap_or(0);
        let new_idx = current_group_idx as i32 + delta;
        if new_idx >= 0 && (new_idx as usize) < groups.len() {
            let target_group = groups[new_idx as usize];
            let offset = cursor_m - groups[current_group_idx].0;
            self.set_cursor_measure((target_group.0 + offset).min(target_group.1));
            self.set_cursor_beat(1.0);
            self.sync_viewport(view_w, view_h);
        }
    }

    fn move_cursor_beat(&mut self, delta: f32, view_w: usize, view_h: usize) {
        let song = self.song.read().unwrap();
        let mut b = self.get_cursor_beat();
        let mut m = self.get_cursor_measure();
        b += delta;
        if b < 1.0 {
            if m > 1 { m -= 1; b = song.time_signature.0 as f32; } 
            else { b = 1.0; }
        } else if b > song.time_signature.0 as f32 {
            if m < song.max_measure { m += 1; b = 1.0; } 
            else { b = song.time_signature.0 as f32; }
        }
        self.set_cursor_measure(m);
        self.set_cursor_beat(b);
        drop(song);
        self.sync_viewport(view_w, view_h);
    }

    fn move_cursor_string(&mut self, delta: i32, view_w: usize, view_h: usize) {
        let groups = self.get_measure_groups(view_w);
        let cursor_m = self.get_cursor_measure();
        let current_group_idx = groups.iter().position(|&(s, e)| cursor_m >= s && cursor_m <= e).unwrap_or(0);
        let new_val = self.cursor_string as i32 + delta;
        if new_val < 0 {
            if current_group_idx > 0 { self.set_cursor_measure(groups[current_group_idx - 1].0); self.cursor_string = 5; }
            else { self.cursor_string = 0; }
        } else if new_val > 5 {
            if current_group_idx + 1 < groups.len() { self.set_cursor_measure(groups[current_group_idx + 1].0); self.cursor_string = 0; }
            else { self.cursor_string = 5; }
        } else { self.cursor_string = new_val as usize; }
        self.sync_viewport(view_w, view_h);
    }

    fn sync_viewport(&mut self, view_w: usize, view_h: usize) {
        let (_, cursor_pos) = self.render_tab_lines_full(true, view_w, view_h);
        let v_buffer = 5;
        if cursor_pos.1 < self.vertical_scroll + v_buffer { self.vertical_scroll = cursor_pos.1.saturating_sub(v_buffer); }
        else if cursor_pos.1 >= self.vertical_scroll + view_h - 2 { self.vertical_scroll = cursor_pos.1 - (view_h - 6); }
        let prefix_w = 4;
        let usable_w = if view_w > prefix_w { view_w - prefix_w - 2 } else { 10 };
        if cursor_pos.0 < self.horizontal_scroll { self.horizontal_scroll = cursor_pos.0; }
        else if cursor_pos.0 >= self.horizontal_scroll + usable_w { self.horizontal_scroll = cursor_pos.0 - (usable_w - 5); }
    }

    fn start_playback(&mut self) {
        let is_playing = Arc::clone(&self.is_playing);
        let song = Arc::clone(&self.song);
        let cursor_m = Arc::clone(&self.cursor_measure);
        let cursor_b_scaled = Arc::clone(&self.cursor_beat_scaled);
        let start_m = self.get_cursor_measure();
        let start_b = self.get_cursor_beat();

        tokio::spawn(async move {
            let mut player = crate::theory::MidiPlayer::new();
            let string_pitches = std::collections::HashMap::from([("e", 64), ("B", 59), ("G", 55), ("D", 50), ("A", 45), ("E", 40)]);
            
            let mut curr_m = start_m;
            let mut curr_b = start_b;

            while is_playing.load(Ordering::SeqCst) {
                let (midi, ts_num, max_m, bpm) = {
                    let song_guard = song.read().unwrap();
                    let max_m = song_guard.max_measure;
                    let ts_num = song_guard.time_signature.0;
                    let bpm = song_guard.bpm;
                    if curr_m > max_m { (vec![], ts_num, max_m, bpm) } 
                    else if let Some(m_data) = song_guard.measures.get(&curr_m) {
                        if let Some(bd) = m_data.get(&curr_b.to_string()) {
                            let midi: Vec<u8> = bd.notes.iter().filter_map(|n| {
                                let base = string_pitches.get(n.string.as_str())?;
                                let f = n.fret.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<u8>().ok()?;
                                Some(base + f)
                            }).collect();
                            (midi, ts_num, max_m, bpm)
                        } else { (vec![], ts_num, max_m, bpm) }
                    } else { (vec![], ts_num, max_m, bpm) }
                };

                if curr_m > max_m { break; }
                
                let ms_per_beat = (60.0 / bpm as f32 * 1000.0) as u64;
                let step = 0.25;
                let ms_per_step = (ms_per_beat as f32 * step) as u64;

                // Update cursor
                cursor_m.store(curr_m, Ordering::SeqCst);
                cursor_b_scaled.store((curr_b * 100.0) as u32, Ordering::SeqCst);

                if !midi.is_empty() { player.play_notes(&midi, ms_per_step); } 
                else { tokio::time::sleep(std::time::Duration::from_millis(ms_per_step)).await; }

                curr_b += step;
                if curr_b > ts_num as f32 { curr_m += 1; curr_b = 1.0; }
            }
            is_playing.store(false, Ordering::SeqCst);
        });
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(f.size());

        let song = self.song.read().unwrap();
        let info_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_GRAY))
            .title(Span::styled(" Info ", Style::default().fg(SECTION_CYAN).add_modifier(Modifier::BOLD)));

        let title = Paragraph::new(format!(
            " TS: {}/{} | BPM: {} | Measure: {} | Beat: {} | String: {} | MIDI: {} ",
            song.time_signature.0, song.time_signature.1, song.bpm, self.get_cursor_measure(), self.get_cursor_beat(),
            ["e", "B", "G", "D", "A", "E"][self.cursor_string], self.midi_port
        )).block(info_block).style(Style::default().fg(FG_LIGHT).bg(BG_DARK));
        f.render_widget(title, chunks[0]);

        let (all_lines, _) = self.render_tab_lines_full(true, chunks[1].width as usize, chunks[1].height as usize);
        let visible_lines: Vec<Line> = all_lines.into_iter().skip(self.vertical_scroll).collect();
        let tab_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_GRAY))
            .title(Span::styled(" Tabs ", Style::default().fg(SECTION_CYAN).add_modifier(Modifier::BOLD)));

        let tab_para = Paragraph::new(visible_lines).block(tab_block).style(Style::default().fg(FG_LIGHT).bg(BG_DARK));
        f.render_widget(tab_para, chunks[1]);

        let help_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_GRAY))
            .title(Span::styled(" Help ", Style::default().fg(SECTION_CYAN).add_modifier(Modifier::BOLD)));

        let help = Paragraph::new(" q: quit | Space: play | h/l: beat | j/k: row | ↑/↓: string | w/b: jump | g/G: start/end | v: voicing | y: yank ")
            .block(help_block).style(Style::default().fg(BEAT_GRAY).bg(BG_DARK));
        f.render_widget(help, chunks[2]);
    }

    fn render_tab_lines_full(&self, styled: bool, view_w: usize, _view_h: usize) -> (Vec<Line<'_>>, (usize, usize)) {
        let mut all_lines = Vec::new();
        let mut cursor_pos = (0, 0);
        let string_names = ["e", "B", "G", "D", "A", "E"];
        let groups = self.get_measure_groups(view_w);
        let performance_markers = ["PM", "PM.", "P.M.", "Harm.", "Harm", "Let Ring"];
        let song = self.song.read().unwrap();
        let cursor_m = self.get_cursor_measure();
        let cursor_b = self.get_cursor_beat();

        for (chunk_m_start, chunk_m_end) in groups {
            let is_cursor_row = cursor_m >= chunk_m_start && cursor_m <= chunk_m_end;
            all_lines.push(Line::from(vec![
                Span::styled("--- MEASURES ", Style::default().fg(FG_LIGHT)),
                Span::styled(format!("{} - {}", chunk_m_start, chunk_m_end), Style::default().fg(SECTION_CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(" ---", Style::default().fg(FG_LIGHT))
            ]));

            for m_num in chunk_m_start..=chunk_m_end {
                if let Some(title) = song.section_headers.get(&m_num) {
                    let is_perf = performance_markers.iter().any(|&m| title.to_uppercase().contains(m));
                    if !is_perf {
                        all_lines.push(Line::from(Span::styled(format!(" # [{}]", title), Style::default().fg(SECTION_CYAN).add_modifier(Modifier::BOLD))));
                    }
                }
            }

            let mut cue_raw = "    ".to_string();
            let mut theory_raw = "    ".to_string();
            let mut string_raw = vec![String::new(); 6];
            let mut beat_raw = "    ".to_string();
            let mut measure_ranges = Vec::new();
            for s in 0..6 { string_raw[s].push_str(&format!("{:2} |", string_names[s])); }
            let mut last_cue_global = String::new();

            for m_num in chunk_m_start..=chunk_m_end {
                let m_start_char = string_raw[0].chars().count();
                for s in 0..6 { string_raw[s].push_str("--"); }
                theory_raw.push_str("  "); beat_raw.push_str("  ");
                if !last_cue_global.is_empty() { cue_raw.push_str("──"); } else { cue_raw.push_str("  "); }

                let mut last_theory = String::new();
                let mut step = 1.0;
                if let Some(m_data) = song.measures.get(&m_num) {
                    for b_str in m_data.keys() {
                        let b: f32 = b_str.parse().unwrap_or(1.0);
                        let frac = (b % 1.0).abs();
                        if frac > 0.001 {
                            let mut s = 1.0;
                            while (frac % s).abs() > 0.001 && s > 0.03125 { s /= 2.0; }
                            if s < step { step = s; }
                        }
                    }
                }

                let num_slots = (song.time_signature.0 as f32 / step).round() as u32;
                let beat_sequence: Vec<f32> = (0..num_slots).map(|i| i as f32 * step + 1.0).collect();

                for b in beat_sequence {
                    let b_str = b.to_string();
                    let label = if b.fract() == 0.0 { b.to_string() } 
                                else if (b.fract() - 0.5).abs() < 0.001 { "&".to_string() }
                                else if (b.fract() - 0.25).abs() < 0.001 { "e".to_string() }
                                else if (b.fract() - 0.75).abs() < 0.001 { "a".to_string() }
                                else { ".".to_string() };

                    let mut col_w = label.len().max(3);
                    let mut bd_opt = None;
                    if let Some(m_data) = song.measures.get(&m_num) {
                        if let Some(bd) = m_data.get(&b_str) {
                            for note in &bd.notes { col_w = col_w.max(note.fret.len()); }
                            bd_opt = Some(bd);
                        }
                    }
                    col_w += 1;

                    if is_cursor_row && m_num == cursor_m && (b - cursor_b).abs() < (step / 2.0) {
                        let current_col_pos = string_raw[0].chars().count();
                        let h_off = if is_cursor_row { self.horizontal_scroll } else { 0 };
                        cursor_pos = (current_col_pos.saturating_sub(4 + h_off), all_lines.len()); 
                    }

                    if let Some(bd) = bd_opt {
                        let mut col = vec!["-".to_string(); 6];
                        for note in &bd.notes {
                            let s_idx = string_names.iter().position(|&s| s == note.string).unwrap_or(0);
                            col[s_idx] = note.fret.clone();
                        }
                        for s in 0..6 { string_raw[s].push_str(&format!("{:width$}", col[s], width = col_w).replace(' ', "-")); }
                        let theory_to_add = if bd.theory != last_theory { &bd.theory } else { "" };
                        if !bd.cue.is_empty() {
                            if bd.cue == last_cue_global { }
                            else { cue_raw.push_str(&bd.cue); last_cue_global = bd.cue.clone(); }
                        } else { last_cue_global = String::new(); }
                        theory_raw.push_str(theory_to_add);
                        beat_raw.push_str(&label);
                        let target_len = string_raw[0].chars().count();
                        let pad_char = if !last_cue_global.is_empty() { '─' } else { ' ' };
                        while cue_raw.chars().count() < target_len { cue_raw.push(pad_char); }
                        while theory_raw.chars().count() < target_len { theory_raw.push(' '); }
                        while beat_raw.chars().count() < target_len { beat_raw.push(' '); }
                        if !theory_to_add.is_empty() { last_theory = bd.theory.clone(); }
                    } else {
                        for s in 0..6 { string_raw[s].push_str(&"-".repeat(col_w)); }
                        beat_raw.push_str(&label);
                        let target_len = string_raw[0].chars().count();
                        let pad_char = if !last_cue_global.is_empty() { '─' } else { ' ' };
                        while cue_raw.chars().count() < target_len { cue_raw.push(pad_char); }
                        while theory_raw.chars().count() < target_len { theory_raw.push(' '); }
                        while beat_raw.chars().count() < target_len { beat_raw.push(' '); }
                    }
                }
                for s in 0..6 { string_raw[s].push('|'); }
                if !last_cue_global.is_empty() { cue_raw.push('|'); } else { cue_raw.push(' '); }
                theory_raw.push(' '); beat_raw.push(' ');
                measure_ranges.push((m_num, m_start_char, string_raw[0].chars().count()));
            }

            let h_off = if is_cursor_row { self.horizontal_scroll } else { 0 };
            let slice_styled_row = |text: String, row_default_style: Style, is_theory: bool, is_cue: bool, is_beat: bool| -> Line {
                let char_count = text.chars().count();
                let prefix = if char_count >= 4 { text.chars().take(4).collect::<String>() } else { text.clone() };
                let body = if char_count > 4 + h_off { text.chars().skip(4+h_off).collect::<String>() } else { String::new() };
                let mut spans = vec![Span::styled(prefix, row_default_style.fg(FG_LIGHT))];
                let body_chars: Vec<char> = body.chars().collect();
                let current_pos = 4 + h_off;
                for &(m_num, m_start, m_end) in &measure_ranges {
                    if m_end <= current_pos { continue; }
                    let start_in_body = m_start.max(current_pos) - current_pos;
                    let end_in_body = m_end.min(current_pos + body_chars.len()) - current_pos;
                    if start_in_body >= body_chars.len() { break; }
                    let segment: String = body_chars[start_in_body..end_in_body].iter().collect();
                    let mut style = row_default_style;
                    if styled && m_num == cursor_m {
                        style = style.bg(HIGHLIGHT_BG).add_modifier(Modifier::BOLD);
                        if is_theory { style = style.fg(CHORD_BLUE); }
                        else if is_cue { style = style.fg(CUE_MAGENTA); }
                        else if is_beat { style = style.fg(CHORD_BLUE); }
                        else { style = style.fg(Color::Yellow); }
                    } else {
                        if is_theory { style = style.fg(CHORD_BLUE); }
                        else if is_cue { style = style.fg(CUE_MAGENTA); }
                        else if is_beat { style = style.fg(BEAT_GRAY); }
                        else if !is_theory && !is_cue && !is_beat { style = style.fg(FRET_ORANGE); }
                    }
                    spans.push(Span::styled(segment, style));
                }
                let mut final_spans = Vec::new();
                let mut total_w = 0;
                for s in spans {
                    let sw = s.content.chars().count();
                    if total_w + sw > view_w {
                        let take = view_w.saturating_sub(total_w);
                        if take > 0 { final_spans.push(Span::styled(s.content.chars().take(take).collect::<String>(), s.style)); }
                        break;
                    }
                    final_spans.push(s); total_w += sw;
                }
                Line::from(final_spans)
            };

            let f_theory = slice_styled_row(theory_raw, Style::default(), true, false, false);
            if f_theory.to_string().chars().count() > 4 && !f_theory.to_string().chars().skip(4).collect::<String>().trim().is_empty() { all_lines.push(f_theory); }
            let f_cue = slice_styled_row(cue_raw, Style::default(), false, true, false);
            if f_cue.to_string().chars().count() > 4 && !f_cue.to_string().chars().skip(4).collect::<String>().trim().is_empty() { all_lines.push(f_cue); }
            for s in 0..6 {
                if styled && is_cursor_row && s == self.cursor_string { cursor_pos.1 = all_lines.len(); }
                all_lines.push(slice_styled_row(string_raw[s].clone(), Style::default(), false, false, false));
            }
            all_lines.push(slice_styled_row(beat_raw, Style::default(), false, false, true));
            all_lines.push(Line::from(""));
        }
        (all_lines, cursor_pos)
    }

    fn cycle_voicing(&mut self) {
        let b_str = self.get_cursor_beat().to_string();
        let mut song = self.song.write().unwrap();
        if let Some(m_map) = song.measures.get_mut(&self.get_cursor_measure()) {
            if let Some(bd) = m_map.get_mut(&b_str) {
                let string_pitches = std::collections::HashMap::from([("e", 64), ("B", 59), ("G", 55), ("D", 50), ("A", 45), ("E", 40)]);
                let midi: Vec<u8> = bd.notes.iter().filter_map(|n| {
                    let base = string_pitches.get(n.string.as_str())?;
                    let f = n.fret.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<u8>().ok()?;
                    Some(base + f)
                }).collect();
                if midi.is_empty() { return; }
                let voicings = crate::theory::find_voicings(&midi);
                if voicings.is_empty() { return; }
                let current: Vec<(String, u8)> = bd.notes.iter().map(|n| {
                    (n.string.clone(), n.fret.chars().filter(|c| c.is_digit(10)).collect::<String>().parse::<u8>().unwrap_or(0))
                }).collect();
                let mut idx = 0;
                for (i, v) in voicings.iter().enumerate() {
                    if v.assignments.len() == current.len() && v.assignments.iter().all(|a| current.contains(a)) {
                        idx = i; break;
                    }
                }
                let next = &voicings[(idx + 1) % voicings.len()];
                bd.notes = next.assignments.iter().map(|(s, f)| crate::parser::NoteEvent { string: s.clone(), fret: f.to_string() }).collect();
            }
        }
    }
}
