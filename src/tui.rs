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

pub struct TuiApp {
    pub song: Song,
    pub cursor_measure: u32,
    pub cursor_beat: f32,
    pub cursor_string: usize,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
}

impl TuiApp {
    pub fn new(song: Song) -> Self {
        Self {
            song,
            cursor_measure: 1,
            cursor_beat: 1.0,
            cursor_string: 0,
            vertical_scroll: 0,
            horizontal_scroll: 0,
        }
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
        
        let m_data = match self.song.measures.get(&m_num) {
            Some(d) if !d.is_empty() => d,
            _ => return 8,
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

        let num_slots = (self.song.time_signature.0 as f32 / step).round() as u32;
        let beat_sequence: Vec<f32> = (0..num_slots).map(|i| (i as f32 * step + 1.0)).collect();

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
        width
    }

    fn get_measure_groups(&self, view_w: usize) -> Vec<(u32, u32)> {
        let mut groups = Vec::new();
        let mut current_m = 1;
        
        while current_m <= self.song.max_measure {
            let mut end_m = current_m;
            let mut current_w = 5;
            
            for m in current_m..=(current_m + 3).min(self.song.max_measure) {
                let m_w = self.calculate_measure_width(m);
                if m > current_m && current_w + m_w + 1 > view_w {
                    break;
                }
                current_w += m_w + 1;
                end_m = m;
            }
            
            let mut has_notes = false;
            for m in current_m..=end_m {
                if let Some(m_data) = self.song.measures.get(&m) {
                    if !m_data.is_empty() { has_notes = true; break; }
                }
            }
            let cursor_in = self.cursor_measure >= current_m && self.cursor_measure <= end_m;
            
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

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') => self.move_cursor_beat(-0.25, view_size.0, view_size.1),
                        KeyCode::Char('l') => self.move_cursor_beat(0.25, view_size.0, view_size.1),
                        KeyCode::Char('j') => self.move_cursor_string(1, view_size.0, view_size.1),
                        KeyCode::Char('k') => self.move_cursor_string(-1, view_size.0, view_size.1),
                        KeyCode::Char('w') => self.move_cursor_measure(1, view_size.0, view_size.1),
                        KeyCode::Char('b') => self.move_cursor_measure(-1, view_size.0, view_size.1),
                        KeyCode::Char('v') => self.cycle_voicing(),
                        KeyCode::Char('y') => self.yank_to_clipboard(),
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
        let new_m = self.cursor_measure as i32 + delta;
        if new_m >= 1 && new_m <= self.song.max_measure as i32 {
            self.cursor_measure = new_m as u32;
            self.cursor_beat = 1.0;
            self.sync_viewport(view_w, view_h);
        }
    }

    fn move_cursor_beat(&mut self, delta: f32, view_w: usize, view_h: usize) {
        self.cursor_beat += delta;
        if self.cursor_beat < 1.0 {
            if self.cursor_measure > 1 {
                self.cursor_measure -= 1;
                self.cursor_beat = self.song.time_signature.0 as f32;
            } else {
                self.cursor_beat = 1.0;
            }
        } else if self.cursor_beat > self.song.time_signature.0 as f32 {
            if self.cursor_measure < self.song.max_measure {
                self.cursor_measure += 1;
                self.cursor_beat = 1.0;
            } else {
                self.cursor_beat = self.song.time_signature.0 as f32;
            }
        }
        self.sync_viewport(view_w, view_h);
    }

    fn move_cursor_string(&mut self, delta: i32, view_w: usize, view_h: usize) {
        let groups = self.get_measure_groups(view_w);
        let current_group_idx = groups.iter().position(|&(s, e)| self.cursor_measure >= s && self.cursor_measure <= e).unwrap_or(0);

        let new_val = self.cursor_string as i32 + delta;
        if new_val < 0 {
            if current_group_idx > 0 {
                self.cursor_measure = groups[current_group_idx - 1].0;
                self.cursor_string = 5;
            } else {
                self.cursor_string = 0;
            }
        } else if new_val > 5 {
            if current_group_idx + 1 < groups.len() {
                self.cursor_measure = groups[current_group_idx + 1].0;
                self.cursor_string = 0;
            } else {
                self.cursor_string = 5;
            }
        } else {
            self.cursor_string = new_val as usize;
        }
        self.sync_viewport(view_w, view_h);
    }

    fn sync_viewport(&mut self, view_w: usize, view_h: usize) {
        let (_, cursor_pos) = self.render_tab_lines_full(true, view_w, view_h);
        
        let v_buffer = 5;
        if cursor_pos.1 < self.vertical_scroll + v_buffer {
            self.vertical_scroll = cursor_pos.1.saturating_sub(v_buffer);
        } else if cursor_pos.1 >= self.vertical_scroll + view_h - 2 {
            self.vertical_scroll = cursor_pos.1 - (view_h - 6);
        }

        let prefix_w = 5;
        let usable_w = if view_w > prefix_w { view_w - prefix_w - 2 } else { 10 };
        if cursor_pos.0 < self.horizontal_scroll {
            self.horizontal_scroll = cursor_pos.0;
        } else if cursor_pos.0 >= self.horizontal_scroll + usable_w {
            self.horizontal_scroll = cursor_pos.0 - (usable_w - 5);
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(f.size());

        let title = Paragraph::new(format!(
            " Tabbit Rust | TS: {}/{} | Measure: {} | Beat: {} | String: {} ",
            self.song.time_signature.0, self.song.time_signature.1, 
            self.cursor_measure, self.cursor_beat,
            ["e", "B", "G", "D", "A", "E"][self.cursor_string]
        ))
        .block(Block::default().borders(Borders::ALL).title(" Info "));
        f.render_widget(title, chunks[0]);

        let (all_lines, _) = self.render_tab_lines_full(true, chunks[1].width as usize, chunks[1].height as usize);
        let visible_lines: Vec<Line> = all_lines.into_iter().skip(self.vertical_scroll).collect();

        let tab_para = Paragraph::new(visible_lines)
            .block(Block::default().borders(Borders::ALL).title(" Tabs "));
        f.render_widget(tab_para, chunks[1]);

        let help = Paragraph::new(" q: quit | h/j/k/l: move | w/b: jump | v: voicing | y: yank ")
            .block(Block::default().borders(Borders::ALL).title(" Help "));
        f.render_widget(help, chunks[2]);
    }

    fn render_tab_lines_full(&self, styled: bool, view_w: usize, _view_h: usize) -> (Vec<Line>, (usize, usize)) {
        let mut all_lines = Vec::new();
        let mut cursor_pos = (0, 0);
        let string_names = ["e", "B", "G", "D", "A", "E"];
        let groups = self.get_measure_groups(view_w);
        let performance_markers = ["PM", "PM.", "P.M.", "Harm.", "Harm", "Let Ring"];

        for (chunk_m_start, chunk_m_end) in groups {
            let is_cursor_row = self.cursor_measure >= chunk_m_start && self.cursor_measure <= chunk_m_end;

            all_lines.push(Line::from(format!("--- MEASURES {} - {} ---", chunk_m_start, chunk_m_end)));

            let mut unique_headers = std::collections::HashSet::new();
            for m_num in chunk_m_start..=chunk_m_end {
                if let Some(title) = self.song.section_headers.get(&m_num) {
                    let is_perf = performance_markers.iter().any(|&m| title.to_uppercase().contains(m));
                    if !is_perf && unique_headers.insert(title) {
                        all_lines.push(Line::from(Span::styled(
                            format!(" # [{}]", title),
                            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                        )));
                    }
                }
            }

            let mut cue_raw = "     ".to_string();
            let mut theory_raw = "     ".to_string();
            let mut string_raw = vec![String::new(); 6];
            let mut beat_raw = "     ".to_string();

            for s in 0..6 { string_raw[s].push_str(&format!("{:2} | ", string_names[s])); }

            let mut last_cue_global = String::new();

            for m_num in chunk_m_start..=chunk_m_end {
                for s in 0..6 { string_raw[s].push_str("--"); }
                theory_raw.push_str("  "); beat_raw.push_str("  ");

                if !last_cue_global.is_empty() {
                    cue_raw.push_str("──");
                } else {
                    cue_raw.push_str("  ");
                }

                let mut last_theory = String::new();
                let mut step = 1.0;
                if let Some(m_data) = self.song.measures.get(&m_num) {
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

                let num_slots = (self.song.time_signature.0 as f32 / step).round() as u32;
                let beat_sequence: Vec<f32> = (0..num_slots).map(|i| (i as f32 * step + 1.0)).collect();

                for b in beat_sequence {
                    let b_str = b.to_string();
                    let label = if b.fract() == 0.0 { b.to_string() } 
                                else if (b.fract() - 0.5).abs() < 0.001 { "&".to_string() }
                                else if (b.fract() - 0.25).abs() < 0.001 { "e".to_string() }
                                else if (b.fract() - 0.75).abs() < 0.001 { "a".to_string() }
                                else { ".".to_string() };

                    let mut col_w = label.len().max(3);
                    let mut bd_opt = None;
                    if let Some(m_data) = self.song.measures.get(&m_num) {
                        if let Some(bd) = m_data.get(&b_str) {
                            for note in &bd.notes { col_w = col_w.max(note.fret.len()); }
                            bd_opt = Some(bd);
                        }
                    }
                    col_w += 1;

                    if is_cursor_row && m_num == self.cursor_measure && (b - self.cursor_beat).abs() < (step / 2.0) {
                        cursor_pos = (string_raw[0].len() - 5, all_lines.len()); 
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
                            if bd.cue == last_cue_global {
                                // Pad
                            } else {
                                cue_raw.push_str(&bd.cue);
                                last_cue_global = bd.cue.clone();
                            }
                        } else {
                            last_cue_global = String::new();
                        }

                        theory_raw.push_str(theory_to_add);
                        beat_raw.push_str(&label);

                        let target_len = string_raw[0].len();
                        let pad_char = if !last_cue_global.is_empty() { '─' } else { ' ' };
                        while cue_raw.chars().count() < target_len { cue_raw.push(pad_char); }
                        while theory_raw.chars().count() < target_len { theory_raw.push(' '); }
                        while beat_raw.chars().count() < target_len { beat_raw.push(' '); }

                        if !theory_to_add.is_empty() { last_theory = bd.theory.clone(); }
                    } else {
                        for s in 0..6 { string_raw[s].push_str(&"-".repeat(col_w)); }
                        beat_raw.push_str(&label);
                        let target_len = string_raw[0].len();
                        let pad_char = if !last_cue_global.is_empty() { '─' } else { ' ' };
                        while cue_raw.chars().count() < target_len { cue_raw.push(pad_char); }
                        while beat_raw.chars().count() < target_len { beat_raw.push(' '); }
                        while theory_raw.chars().count() < target_len { theory_raw.push(' '); }
                    }
                }
                for s in 0..6 { string_raw[s].push('|'); }
                if !last_cue_global.is_empty() {
                    cue_raw.push('|');
                } else {
                    cue_raw.push(' ');
                }
                theory_raw.push(' '); beat_raw.push(' ');
            }

            let h_off = if is_cursor_row { self.horizontal_scroll } else { 0 };
            let slice = |s: String| {
                let prefix = if s.chars().count() >= 5 { s.chars().take(5).collect::<String>() } else { s.clone() };
                let body = if s.chars().count() > 5 + h_off { s.chars().skip(5+h_off).collect::<String>() } else { String::new() };
                let mut combined = format!("{}{}", prefix, body);
                if styled && combined.chars().count() > view_w { combined = combined.chars().take(view_w).collect(); }
                combined
            };

            let f_theory = slice(theory_raw);
            let f_cue = slice(cue_raw);

            if f_theory.chars().count() > 5 && !f_theory[5..].trim().is_empty() {
                all_lines.push(Line::from(f_theory));
            }
            if f_cue.chars().count() > 5 && !f_cue[5..].trim().is_empty() {
                all_lines.push(Line::from(Span::styled(f_cue, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))));
            }

            for s in 0..6 {
                let line_str = slice(string_raw[s].clone());
                let style = if styled && is_cursor_row && s == self.cursor_string {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else { Style::default() };
                if styled && is_cursor_row && s == self.cursor_string {
                    cursor_pos.1 = all_lines.len();
                }
                all_lines.push(Line::from(Span::styled(line_str, style)));
            }
            all_lines.push(Line::from(slice(beat_raw)));
            all_lines.push(Line::from(""));
        }
        (all_lines, cursor_pos)
    }

    fn cycle_voicing(&mut self) {
        let b_str = self.cursor_beat.to_string();
        if let Some(m_map) = self.song.measures.get_mut(&self.cursor_measure) {
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
