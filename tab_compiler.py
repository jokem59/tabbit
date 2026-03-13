import re

class MusicTheory:
    NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    FLAT_MAP = {
        'DB': 'C#', 'EB': 'D#', 'GB': 'F#', 'AB': 'G#', 'BB': 'A#',
        'CB': 'B', 'FB': 'E', 'E#': 'F', 'B#': 'C'
    }
    
    @staticmethod
    def get_note_name(midi_pitch):
        return MusicTheory.NOTE_NAMES[midi_pitch % 12]

    @staticmethod
    def normalize_root(root):
        if not root: return None
        # Handle cases like 'Gb' -> 'GB' or 'F#' -> 'F#'
        root = root.upper()
        # Robustly extract the base note and its accidental
        match = re.match(r"^([A-G][#B]?)", root)
        if not match: return None
        clean_root = match.group(1)
        return MusicTheory.FLAT_MAP.get(clean_root, clean_root)

    @staticmethod
    def analyze_chord(notes, root_hint=None):
        """
        Takes a list of MIDI pitches and an optional root hint (e.g., 'G').
        Returns chord name + intervals relative to the root hint.
        """
        if not notes: return ""
        # Filter out muted notes (None)
        notes = sorted(list(set([n for n in notes if n is not None])))
        if not notes: return ""
        
        normalized_root = MusicTheory.normalize_root(root_hint)
        
        if normalized_root:
            # Find the MIDI offset for the hint (e.g., 'G' -> 7)
            root_index = MusicTheory.NOTE_NAMES.index(normalized_root)
            root_pitch_class = root_index
        else:
            # Fallback to lowest note if no hint
            root_pitch_class = notes[0] % 12
            root_hint = MusicTheory.get_note_name(notes[0])

        intervals = [(n - root_pitch_class) % 12 for n in notes]
        
        # Interval Mapping
        interval_names = {0: '1', 1: 'b2', 2: '2', 3: 'b3', 4: '3', 5: '4', 6: 'b5', 7: '5', 8: 'b6', 9: '6', 10: 'b7', 11: '7'}
        readable_ints = "-".join([interval_names[i] for i in intervals])
        
        # Single Note Functional Shorthand: Just show the hint
        if len(notes) == 1:
            return root_hint

        # Determine Chord Quality
        quality = ""
        int_set = set(intervals)

        if {0, 4, 7, 10}.issubset(int_set): quality = "Dom7"
        elif {0, 4, 7}.issubset(int_set): quality = "Major"
        elif {0, 3, 7}.issubset(int_set): quality = "Minor"
        elif int_set == {0, 7}: quality = "Power"
        else: quality = "Complex/Lead"
        
        # Check for Inversion (Is the root the lowest note?)
        is_inversion = (notes[0] % 12) != root_pitch_class
        inversion_label = f" (Inversion: /{MusicTheory.get_note_name(notes[0])})" if is_inversion else ""
        
        return f"{root_hint} {quality} [{readable_ints}]{inversion_label}"

    @staticmethod
    def suggest_embellishment(notes, root_hint=None):
        """Suggests common guitar moves with associated 'feelings'."""
        if not notes: return ""
        # Filter out muted notes
        notes = [n for n in notes if n is not None]
        if not notes: return ""
        
        # Resolve root
        normalized_root = MusicTheory.normalize_root(root_hint)
        if normalized_root:
            try:
                root_pc = MusicTheory.NOTE_NAMES.index(normalized_root)
            except ValueError: return ""
        else:
            root_pc = notes[0] % 12
            
        intervals = set([(n - root_pc) % 12 for n in notes])
        
        # 1. Power Chord (1-5)
        if {0, 7}.issubset(intervals) and len(intervals & {3, 4}) == 0:
            if 2 not in intervals: return "💡 Tip: Add a '2' (9) for a modern, open-voiced 'big' sound."
            return "💡 Tip: Add a 'b3' (minor) for darkness, or '3' (major) for brightness."

        # 2. Major Context (1-3-5)
        if {0, 4}.issubset(intervals):
            if 2 not in intervals and 5 not in intervals:
                return "💡 Tip: Try a 'sus2' (2) for a jangly, airy feel."
            if 5 not in intervals:
                return "💡 Tip: Try a 'sus4' (5) for a church-like, regal resolution."
            if 11 not in intervals:
                return "💡 Tip: Add a 'maj7' (11) for a lush, sophisticated 'dreamy' color."
            if 9 not in intervals:
                return "💡 Tip: Add a '6' (9) for a sweet, country-style 'western' feel."
            if 6 not in intervals:
                return "💡 Tip: Try a '#11' (6) for a spacey, Lydian soundtrack vibe."

        # 3. Minor Context (1-b3-5)
        if {0, 3}.issubset(intervals):
            if 2 not in intervals:
                return "💡 Tip: Hammer the 'b3' from the '2' for a soulful, bluesy minor feel."
            if 10 not in intervals:
                return "💡 Tip: Add a 'b7' (10) for a moody, jazz-ballad texture."
            if 9 not in intervals:
                return "💡 Tip: Add a '6' (9) for a 'Dorian' funk/fusion flavor."

        # 4. Dominant Context (1-3-5-b7)
        if {0, 4, 10}.issubset(intervals):
            if 3 not in intervals: return "💡 Tip: Try a '#9' (3) for that gritty 'Jimi' Hendrix tension."
            if 1 not in intervals: return "💡 Tip: Try a 'b9' (1) for a tense, dark cinematic resolution."

        return "💡 Tip: Experiment with open strings for a ringing, resonant drone."

class TabCompiler:
    def __init__(self, time_sig=(4,4)):
        self.ts_num, self.ts_den = time_sig
        self.string_pitches = [64, 59, 55, 50, 45, 40] # e B G D A E
        self.string_names = ['e', 'B', 'G', 'D', 'A', 'E']
        self.sections = []
        self.active_section = {"name": None, "measures": {}}
        self.sections.append(self.active_section)

    def parse_line(self, line):
        line = line.strip()
        if not line: return

        # Check for Section Header (e.g., # [Intro])
        section_match = re.match(r"^#\s*\[(.*)\]", line)
        if section_match:
            name = section_match.group(1)
            # Start new section (rename first one if it's empty)
            if not self.active_section["measures"] and self.active_section["name"] is None:
                self.active_section["name"] = name
            else:
                self.active_section = {"name": name, "measures": {}}
                self.sections.append(self.active_section)
            return

        # Updated Regex: 
        # 1. Support case-insensitive 'OVER'
        # 2. Allow any non-whitespace in root hints (e.g., F#m, Asus4) by using \S+
        # 3. Use lookahead (?=...) to ensure notes don't swallow OVER or cues
        pattern = r"^M(\d+):(\d+\.?\d*)\s*\|\s*(.*?)(?=\s+[Oo][Vv][Ee][Rr]\s+|\s*#|$)(?:\s+[Oo][Vv][Ee][Rr]\s+(\S+))?(?:\s*#\s*\[(.*)\])?$"
        match = re.match(pattern, line)
        
        if not match:
            # Diagnostic help for the user
            if not re.match(r"^M\d+:", line):
                # Just skip unrelated comments
                if line.startswith("#"): return
                raise ValueError("Line must start with 'M[Measure]:[Beat]'. Example: 'M1:1 | ...'")
            if "|" not in line:
                raise ValueError("Missing mandatory pipe '|' after the beat. Example: 'M1:1 | e:0'")
            raise ValueError("Invalid shorthand syntax. Check notes or OVER/cue format.")

        m_num = int(match.group(1))
        b_num = float(match.group(2))
        notes_raw = match.group(3).strip().split()
        root_hint = match.group(4)
        cue = match.group(5) if match.group(5) else ""
        
        if m_num not in self.active_section["measures"]: self.active_section["measures"][m_num] = {}
        
        note_events = []
        midi_pitches = []
        for n in notes_raw:
            if ":" not in n:
                raise ValueError(f"Invalid note format '{n}'. Expected 'String:Fret' (e.g., 'e:5')")
            try:
                s_name, fret_val = n.split(":")
                if s_name not in self.string_names:
                    raise ValueError(f"Invalid string name '{s_name}'. Use e, B, G, D, A, or E.")
                s_idx = self.string_names.index(s_name)
                
                fret_match = re.search(r"\d+", fret_val)
                if fret_match:
                    fret_num = int(fret_match.group())
                    midi_pitches.append(self.string_pitches[s_idx] + fret_num)
                    note_events.append({"string": s_idx, "fret": fret_val})
                elif "x" in fret_val.lower():
                    midi_pitches.append(None) # Muted/Dead note
                    note_events.append({"string": s_idx, "fret": fret_val})
                else:
                    raise ValueError(f"Invalid fret value '{fret_val}'. Use numbers or 'x'.")
            except Exception as e:
                if isinstance(e, ValueError): raise e
                raise ValueError(f"Error parsing note '{n}': {e}")
            
        theory = ""
        tip = ""
        if root_hint:
            theory = MusicTheory.analyze_chord(midi_pitches, root_hint)
            tip = MusicTheory.suggest_embellishment(midi_pitches, root_hint)
        
        self.active_section["measures"][m_num][b_num] = {
            "events": note_events,
            "cue": cue,
            "theory": theory,
            "tip": tip
        }

    def _place_multiline(self, lines, text, start_col):
        """Places text in the first available line at start_col without overlapping."""
        if not text: return
        
        for i in range(len(lines)):
            line = lines[i]
            # Ensure the line is long enough to check
            if len(line) <= start_col:
                lines[i] = line.ljust(start_col) + text
                return
            
            # Check if there's space (all characters at start_col to end of text are spaces)
            # We check a slightly wider area to ensure a small gap between labels on the same line
            target_area = line[start_col:start_col + len(text) + 2]
            if target_area.strip() == "":
                # Pad if necessary
                if len(line) < start_col:
                    lines[i] = line.ljust(start_col) + text
                else:
                    # String slice replacement
                    new_line = line[:start_col] + text + line[start_col + len(text):]
                    lines[i] = new_line
                return
        
        # If no space found in existing lines, add a new one
        lines.append(" " * start_col + text)

    def compile(self, measures_per_line=4, show_tips=True):
        output = []
        
        for section in self.sections:
            m_keys = sorted(section["measures"].keys())
            if not m_keys: continue
            
            # Fill gaps ONLY within the section
            measure_numbers = list(range(min(m_keys), max(m_keys) + 1))
            
            idx = 0
            while idx < len(measure_numbers):
                # 1. Peek at resolution of next few measures to decide chunk size
                peek_indices = measure_numbers[idx:idx + measures_per_line]
                highest_res = 1.0
                for m_num in peek_indices:
                    m_data = section["measures"].get(m_num, {})
                    for b in m_data:
                        f = round(b % 1, 4)
                        if f == 0: continue
                        
                        # Check resolution from largest to smallest to avoid shadowing
                        if f % 0.5 == 0: res = 0.5
                        elif f % 0.25 == 0: res = 0.25
                        elif f % 0.125 == 0: res = 0.125
                        else: res = 0.0625
                        
                        highest_res = min(highest_res, res)

                # Dynamic chunk size: 16th notes -> 2 per line, 32nd -> 1 per line
                actual_mpl = measures_per_line
                if highest_res <= 0.125: actual_mpl = 1
                elif highest_res <= 0.25: actual_mpl = min(measures_per_line, 2)

                # Ensure we don't go out of bounds or create an infinite loop
                chunk_indices = measure_numbers[idx:idx + actual_mpl]

                # Chunk Header: Include section name if it exists
                sec_label = f": {section['name']}" if section["name"] else ""
                output.append(f"\n--- MEASURES {chunk_indices[0]} - {chunk_indices[-1]} ({self.ts_num}/{self.ts_den}){sec_label} ---")

                # Prepare lines for this chunk
                chunk_lines = [f"{n} |" for n in self.string_names]
                chunk_theory_lines = ["   "]
                chunk_cue_lines = ["   "]
                chunk_beats = "   "
                chunk_tips = []
                chunk_cues = [] # List of (col, text)
                last_theory = None

                # 1. Determine GLOBAL resolution (step) and GLOBAL max fret width for THIS CHUNK
                global_step = 1.0
                global_max_fret_w = 1

                def get_min_resolution(b):
                    f = round(b % 1, 4)
                    if f == 0: return 1.0
                    if f % 0.5 == 0: return 0.5
                    if f % 0.25 == 0: return 0.25
                    if f % 0.125 == 0: return 0.125
                    return 0.0625

                for m_num in chunk_indices:
                    m_data = section["measures"].get(m_num, {})
                    for b, b_data in m_data.items():
                        global_step = min(global_step, get_min_resolution(b))
                        for e in b_data['events']:
                            global_max_fret_w = max(global_max_fret_w, len(e['fret']))

                # 2. Determine global_slot_w for even spacing across the chunk
                global_slot_w = global_max_fret_w + 1
                if global_step >= 0.5: global_slot_w = max(global_slot_w, 4)
                elif global_step == 0.25: global_slot_w = max(global_slot_w, 3)
                else: global_slot_w = max(global_slot_w, 2)

                current_chunk_col = 3

                for m_num in chunk_indices:
                    m_data = section["measures"].get(m_num, {})

                    # 1. Determine resolution (step) greedily
                    def get_resolution(b):
                        f = round(b % 1, 4)
                        if f == 0: return 1.0
                        if f % 0.5 == 0: return 0.5
                        if f % 0.25 == 0: return 0.25
                        if f % 0.125 == 0: return 0.125
                        return 0.0625

                    step = 1.0
                    for b in m_data:
                        step = min(step, get_resolution(b))

                    # Use global_slot_w for consistency
                    slot_w = global_slot_w

                    num_slots = int(round(self.ts_num / step))
                    beat_sequence = [round(j * step + 1, 4) for j in range(num_slots)]

                    measure_output = ["" for _ in range(6)]
                    m_beat_line = ""
                    m_cues = [] # (col, text) for THIS measure

                    # Reset last_theory per measure to ensure it shows up at the start of each bar for visibility
                    last_theory = None

                    beat_offset = 0
                    for b in beat_sequence:
                        frac = round(b % 1, 4)
                        label = ""
                        if frac == 0: label = str(int(b))
                        elif frac == 0.5: label = "&"
                        elif frac == 0.25: label = "e"
                        elif frac == 0.75: label = "a"

                        column = ["-"] * 6
                        beat_col = current_chunk_col + 2 + beat_offset

                        if b in m_data:
                            beat_data = m_data[b]
                            for e in beat_data['events']:
                                column[e['string']] = e['fret']

                            # Place Theory (Deduplicated)
                            if beat_data['theory'] and beat_data['theory'] != last_theory:
                                self._place_multiline(chunk_theory_lines, beat_data['theory'], beat_col)
                                last_theory = beat_data['theory']

                            # Collect Cues for this measure
                            if beat_data['cue']:
                                m_cues.append((beat_col, beat_data['cue']))

                            # Collect Tips (Deduplicated)
                            if beat_data['tip'] and beat_data['tip'] not in chunk_tips:
                                chunk_tips.append(beat_data['tip'])

                        for s_idx in range(6):
                            measure_output[s_idx] += column[s_idx].ljust(slot_w, "-")

                        m_beat_line += label.ljust(slot_w)
                        beat_offset += slot_w

                    for s_idx in range(6):
                        # Start each measure with '--' padding
                        chunk_lines[s_idx] += "--" + measure_output[s_idx] + "|"

                    chunk_beats += "  " + m_beat_line + " "

                    # Process measure-specific cues into Range Brackets
                    m_groups = []
                    for col, txt in m_cues:
                        if not m_groups or m_groups[-1][0] != txt:
                            m_groups.append([txt, [(col, txt)]])
                        else:
                            m_groups[-1][1].append((col, txt))

                    for txt, occurrences in m_groups:
                        start_col = occurrences[0][0]
                        label = f"[{txt}]"
                        if len(occurrences) > 1:
                            end_col = occurrences[-1][0]
                            dash_count = max(0, end_col - start_col - len(label))
                            line = label + "-" * dash_count + "|"
                            self._place_multiline(chunk_cue_lines, line, start_col)
                        else:
                            self._place_multiline(chunk_cue_lines, label, start_col)

                    # Ensure meta-lines are padded to match the measure boundary
                    measure_w = (num_slots * slot_w) + 3
                    for lines in [chunk_theory_lines, chunk_cue_lines]:
                        for i in range(len(lines)):
                            lines[i] = lines[i].ljust(current_chunk_col + measure_w)

                    current_chunk_col += measure_w

                # Move to next chunk
                idx += actual_mpl

                # Final assembly for this chunk
                output.extend(chunk_cue_lines)
                # Only add theory lines if they contain actual text (ignoring initial padding)
                if any(line.strip() for line in chunk_theory_lines):
                    output.extend(chunk_theory_lines)
                output.extend(chunk_lines)
                output.append(chunk_beats)
                
                # Tips (Deduplicated across chunk)
                if show_tips:
                    for tip in chunk_tips:
                        output.append(f"   {tip}")
        
        return "\n".join(output)

if __name__ == "__main__":
    compiler = TabCompiler(time_sig=(6, 4))
    
    # Simulating a file read
    shorthand_input = [
        "M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 # [E Major Clean]",
        "M1:5 | A:3 D:5 G:5 B:5 # [C Major Triad]",
        "M2:1 | G:2h4 B:3 # [Lead Lick]"
    ]
    
    for line in shorthand_input:
        compiler.parse_line(line)
        
    print(compiler.compile())
