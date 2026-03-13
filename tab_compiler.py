import re

class MusicTheory:
    NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    
    @staticmethod
    def get_note_name(midi_pitch):
        return MusicTheory.NOTE_NAMES[midi_pitch % 12]

    @staticmethod
    def analyze_chord(notes, root_hint=None):
        """
        Takes a list of MIDI pitches and an optional root hint (e.g., 'G').
        Returns chord name + intervals relative to the root hint.
        """
        if not notes: return ""
        notes = sorted(list(set(notes)))
        
        if root_hint:
            # Find the MIDI offset for the hint (e.g., 'G' -> 7)
            root_index = MusicTheory.NOTE_NAMES.index(root_hint.upper())
            root_pitch_class = root_index
        else:
            # Fallback to lowest note if no hint
            root_pitch_class = notes[0] % 12
            root_hint = MusicTheory.get_note_name(notes[0])

        intervals = [(n - root_pitch_class) % 12 for n in notes]
        
        # Interval Mapping
        interval_names = {0: '1', 1: 'b2', 2: '2', 3: 'b3', 4: '3', 5: '4', 6: 'b5', 7: '5', 8: 'b6', 9: '6', 10: 'b7', 11: '7'}
        readable_ints = "-".join([interval_names[i] for i in intervals])
        
        # Determine Chord Quality
        quality = "Complex/Lead"
        int_set = set(intervals)
        if {0, 4, 7}.issubset(int_set): quality = "Major"
        elif {0, 3, 7}.issubset(int_set): quality = "Minor"
        elif {0, 7}.issubset(int_set) and len(intervals) == 2: quality = "Power"
        elif {0, 4, 7, 10}.issubset(int_set): quality = "Dom7"
        
        # Check for Inversion (Is the root the lowest note?)
        is_inversion = (notes[0] % 12) != root_pitch_class
        inversion_label = f" (Inversion: /{MusicTheory.get_note_name(notes[0])})" if is_inversion else ""
        
        return f"{root_hint} {quality} [{readable_ints}]{inversion_label}"

    @staticmethod
    def suggest_embellishment(notes, root_hint=None):
        """Suggests a common guitar move based on detected chord and root."""
        if not notes: return ""
        
        # Resolve root
        if root_hint:
            root_pc = MusicTheory.NOTE_NAMES.index(root_hint.upper())
        else:
            root_pc = notes[0] % 12
            
        intervals = [(n - root_pc) % 12 for n in notes]
        
        if 4 in intervals: # Major context
            if 2 not in intervals: return "💡 Tip: Try a '2' (sus2) for a jangly feel."
            return "💡 Tip: Try adding a 'maj7' (interval 11) for jazzier color."
        if 3 in intervals: # Minor context
            return "💡 Tip: Hammer the 'b3' from the '2' for a bluesy minor feel."
            
        return ""

class TabCompiler:
    def __init__(self, time_sig=(4,4)):
        self.ts_num, self.ts_den = time_sig
        self.string_pitches = [64, 59, 55, 50, 45, 40] # e B G D A E
        self.string_names = ['e', 'B', 'G', 'D', 'A', 'E']
        self.measures = {}

    def parse_line(self, line):
        # Improved Regex to strictly separate notes, root hints, and cues
        # Group 1: Measure, Group 2: Beat (now supports decimals), Group 3: Notes, Group 4: Root Hint, Group 5: Cue
        pattern = r'^M(\d+):(\d+\.?\d*)\s*\|\s*(.*?)(?:\s+OVER\s+([A-Ga-g#]+))?(?:\s*#\s*\[(.*)\])?$'
        match = re.match(pattern, line.strip())
        if not match: return
        
        m_num = int(match.group(1))
        b_num = float(match.group(2))
        notes_raw = match.group(3).strip().split()
        root_hint = match.group(4)
        cue = match.group(5) if match.group(5) else ""
        
        if m_num not in self.measures: self.measures[m_num] = {}
        
        note_events = []
        midi_pitches = []
        for n in notes_raw:
            s_name, fret_val = n.split(':')
            s_idx = self.string_names.index(s_name)
            fret_num = int(re.search(r'\d+', fret_val).group())
            midi_pitches.append(self.string_pitches[s_idx] + fret_num)
            note_events.append({'string': s_idx, 'fret': fret_val})
            
        theory = MusicTheory.analyze_chord(midi_pitches, root_hint)
        tip = MusicTheory.suggest_embellishment(midi_pitches, root_hint)
        
        self.measures[m_num][b_num] = {
            "events": note_events,
            "cue": cue,
            "theory": theory,
            "tip": tip
        }

    def compile(self, measures_per_line=4):
        output = []
        measure_numbers = sorted(self.measures.keys())
        
        for i in range(0, len(measure_numbers), measures_per_line):
            chunk_indices = measure_numbers[i:i + measures_per_line]
            
            # Chunk Header
            output.append(f"\n--- MEASURES {chunk_indices[0]} - {chunk_indices[-1]} ({self.ts_num}/{self.ts_den}) ---")
            
            # Prepare lines for this chunk
            chunk_lines = [f"{n} |-" for n in self.string_names]
            chunk_theory = "    "
            chunk_cue = "    "
            
            for m_num in chunk_indices:
                m_data = self.measures[m_num]
                
                # Determine resolution for THIS measure
                step = 1.0
                for b in m_data.keys():
                    frac = b % 1
                    if frac == 0: continue
                    if frac % 0.125 == 0: step = min(step, 0.125)
                    elif frac % 0.25 == 0: step = min(step, 0.25)
                    elif frac % 0.5 == 0: step = min(step, 0.5)
                    else: step = min(step, 0.0625)
                
                num_slots = int(round(self.ts_num / step))
                beat_sequence = [round(j * step + 1, 4) for j in range(num_slots)]
                
                measure_output = ["" for _ in range(6)]
                m_theory_line = ""
                m_cue_line = ""
                
                for b in beat_sequence:
                    if b in m_data:
                        beat_data = m_data[b]
                        
                        # Notes
                        column = ["-"] * 6
                        for e in beat_data['events']:
                            column[e['string']] = e['fret']
                        
                        max_w = max(len(c) for c in column)
                        for s_idx in range(6):
                            measure_output[s_idx] += column[s_idx].ljust(max_w, '-') + "--"
                        
                        # Align Theory/Cues to the width of the note column
                        col_width = max_w + 2
                        m_theory_line += beat_data['theory'].ljust(col_width)
                        cue_txt = f"[{beat_data['cue']}]" if beat_data['cue'] else ""
                        m_cue_line += cue_txt.ljust(col_width)
                    else:
                        # Empty slot
                        fill = "-" * 4
                        for s_idx in range(6): measure_output[s_idx] += fill
                        m_theory_line += " " * len(fill)
                        m_cue_line += " " * len(fill)
                
                # Combine this measure into the chunk lines
                for s_idx in range(6):
                    chunk_lines[s_idx] += measure_output[s_idx] + "|"
                
                # Add measure boundary to theory/cue lines
                chunk_theory += m_theory_line + " "
                chunk_cue += m_cue_line + " "
            
            output.append(chunk_cue)
            output.extend(chunk_lines)
            output.append(chunk_theory)
            
            # Collective Tips for the chunk
            for m_num in chunk_indices:
                for b in self.measures[m_num]:
                    if self.measures[m_num][b]['tip']:
                        output.append(f"   (M{m_num}) {self.measures[m_num][b]['tip']}")
                        
        return "\n".join(output)
                    
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
