import re
from enum import Enum, auto

# --- COMPILER INFRASTRUCTURE ---

class TokenType(Enum):
    TS = auto()
    MEASURE = auto()
    BEAT = auto()
    STRING = auto()
    FRET = auto()
    OVER = auto()
    CUE = auto()
    REPEAT = auto()
    LBRACE = auto()
    RBRACE = auto()
    EQUALS = auto()
    IDENTIFIER = auto()
    NUMBER = auto()
    EOF = auto()

class Token:
    def __init__(self, type, value, line, column):
        self.type = type
        self.value = value
        self.line = line
        self.column = column
    def __repr__(self):
        return f"Token({self.type}, {repr(self.value)}, {self.line}:{self.column})"

class Lexer:
    def __init__(self, text):
        self.text = text
        self.pos = 0
        self.line = 1
        self.column = 1

    def error(self, message):
        raise Exception(f"Lexer Error at {self.line}:{self.column} - {message}")

    def peek(self, n=0):
        if self.pos + n >= len(self.text): return None
        return self.text[self.pos + n]

    def advance(self):
        char = self.peek()
        self.pos += 1
        if char == '\n':
            self.line += 1
            self.column = 1
        else:
            self.column += 1
        return char

    def get_tokens(self):
        while self.pos < len(self.text):
            char = self.peek()

            if char.isspace():
                self.advance()
                continue

            if char == '#': # Comments / Cues
                start_col = self.column
                self.advance()
                if self.peek() == '[':
                    self.advance()
                    cue_val = ""
                    while self.peek() and self.peek() != ']':
                        cue_val += self.advance()
                    if self.peek() == ']': self.advance()
                    yield Token(TokenType.CUE, cue_val, self.line, start_col)
                else:
                    while self.peek() and self.peek() != '\n':
                        self.advance()
                continue

            if char == 'M' and self.peek(1) and self.peek(1).isdigit():
                start_col = self.column
                self.advance() # M
                yield Token(TokenType.MEASURE, 'M', self.line, start_col)
                continue

            if char.isalpha():
                start_col = self.column
                word = ""
                while self.peek() and (self.peek().isalnum() or self.peek() == '_'):
                    word += self.advance()
                
                if word == "TS": yield Token(TokenType.TS, word, self.line, start_col)
                elif word == "OVER": yield Token(TokenType.OVER, word, self.line, start_col)
                elif word == "REPEAT": yield Token(TokenType.REPEAT, word, self.line, start_col)
                elif word in ['e', 'B', 'G', 'D', 'A', 'E']:
                    yield Token(TokenType.STRING, word, self.line, start_col)
                else:
                    yield Token(TokenType.IDENTIFIER, word, self.line, start_col)
                continue

            if char.isdigit():
                start_col = self.column
                num_str = ""
                while self.peek() and (self.peek().isdigit() or self.peek() in ['.', 'h', 'p', 's', 'b', 'v', 'r']):
                    num_str += self.advance()
                yield Token(TokenType.NUMBER, num_str, self.line, start_col)
                continue

            if char == ':': yield Token(TokenType.BEAT, self.advance(), self.line, self.column-1)
            elif char == '|': yield Token(TokenType.FRET, self.advance(), self.line, self.column-1)
            elif char == '{': yield Token(TokenType.LBRACE, self.advance(), self.line, self.column-1)
            elif char == '}': yield Token(TokenType.RBRACE, self.advance(), self.line, self.column-1)
            elif char == '=': yield Token(TokenType.EQUALS, self.advance(), self.line, self.column-1)
            elif char == '/': yield Token(TokenType.FRET, self.advance(), self.line, self.column-1) # Using FRET token for '/' too
            elif char == '(': yield Token(TokenType.LBRACE, self.advance(), self.line, self.column-1)
            elif char == ')': yield Token(TokenType.RBRACE, self.advance(), self.line, self.column-1)
            else:
                self.error(f"Unexpected character: {char}")
        
        yield Token(TokenType.EOF, None, self.line, self.column)

# --- MUSIC THEORY ENGINE ---

class MusicTheory:
    NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    
    @staticmethod
    def get_note_name(midi_pitch):
        return MusicTheory.NOTE_NAMES[midi_pitch % 12]

    @staticmethod
    def analyze_chord(notes, root_hint=None):
        if not notes: return ""
        notes = sorted(list(set(notes)))
        
        # Resolve root
        root_pitch_class = (MusicTheory.NOTE_NAMES.index(root_hint.upper()) if root_hint else notes[0] % 12)
        root_name = root_hint.upper() if root_hint else MusicTheory.get_note_name(notes[0])
        
        # If it's a single note and we have a hint, just show the hint
        if len(notes) == 1 and root_hint:
            return root_name

        intervals = [(n - root_pitch_class) % 12 for n in notes]
        interval_names = {0: '1', 1: 'b2', 2: '2', 3: 'b3', 4: '3', 5: '4', 6: 'b5', 7: '5', 8: 'b6', 9: '6', 10: 'b7', 11: '7'}
        readable_ints = "-".join([interval_names[i] for i in intervals])
        
        quality = "Lead"
        int_set = set(intervals)
        if {0, 4, 7}.issubset(int_set): quality = "Maj"
        elif {0, 3, 7}.issubset(int_set): quality = "Min"
        elif {0, 7}.issubset(int_set) and len(intervals) == 2: quality = "Pwr"
        
        return f"{root_name}{quality} [{readable_ints}]"

# --- COMPILER / EMITTER ---

class TabCompiler:
    def __init__(self, time_sig=(4,4)):
        self.ts_num, self.ts_den = time_sig
        self.string_pitches = [64, 59, 55, 50, 45, 40] # e B G D A E
        self.string_names = ['e', 'B', 'G', 'D', 'A', 'E']
        self.measures = {}
        self.macros = {}
        self.measure_offset = 0

    def compile_text(self, text):
        tokens = list(Lexer(text).get_tokens())
        self.pos = 0
        self.tokens = tokens
        self.parse_program()

    def peek(self): return self.tokens[self.pos]
    def advance(self):
        tok = self.tokens[self.pos]
        self.pos += 1
        return tok

    def expect(self, type, message=None):
        tok = self.peek()
        if tok.type != type:
            msg = message or f"Expected {type}, found {tok.type} ('{tok.value}')"
            raise Exception(f"Syntax Error at {tok.line}:{tok.column} - {msg}")
        return self.advance()

    def parse_program(self):
        while self.peek().type != TokenType.EOF:
            tok = self.peek()
            if tok.type == TokenType.TS:
                self.advance()
                self.ts_num = int(float(self.expect(TokenType.NUMBER).value))
                self.expect(TokenType.FRET) # /
                self.ts_den = int(float(self.expect(TokenType.NUMBER).value))
            elif tok.type == TokenType.MEASURE:
                self.parse_measure_event()
            elif tok.type == TokenType.REPEAT:
                self.parse_repeat_block()
            elif tok.type == TokenType.IDENTIFIER:
                name = self.advance().value
                if self.peek().type == TokenType.EQUALS:
                    self.advance()
                    self.expect(TokenType.LBRACE)
                    start = self.pos
                    depth = 1
                    while depth > 0:
                        t = self.advance()
                        if t.type == TokenType.LBRACE: depth += 1
                        if t.type == TokenType.RBRACE: depth -= 1
                    self.macros[name] = self.tokens[start:self.pos-1]
                else:
                    if name in self.macros:
                        saved_tokens = self.tokens
                        saved_pos = self.pos
                        self.tokens = self.macros[name] + [Token(TokenType.EOF, None, 0, 0)]
                        self.pos = 0
                        self.parse_program()
                        self.tokens = saved_tokens
                        self.pos = saved_pos
            else:
                self.advance()

    def parse_repeat_block(self):
        self.advance() # REPEAT
        count = int(float(self.expect(TokenType.NUMBER).value))
        self.expect(TokenType.LBRACE)
        
        start_pos = self.pos
        depth = 1
        while depth > 0:
            t = self.advance()
            if t.type == TokenType.LBRACE: depth += 1
            if t.type == TokenType.RBRACE: depth -= 1
        end_pos = self.pos - 1
        
        block_tokens = self.tokens[start_pos:end_pos]
        
        for i in range(count):
            saved_tokens = self.tokens
            saved_pos = self.pos
            self.tokens = block_tokens + [Token(TokenType.EOF, None, 0, 0)]
            self.pos = 0
            self.parse_program()
            self.tokens = saved_tokens
            self.pos = saved_pos
            
            if i < count - 1:
                new_max = max(self.measures.keys()) if self.measures else 0
                self.measure_offset = new_max 

    def parse_measure_event(self):
        self.advance() # M
        m_num = int(float(self.expect(TokenType.NUMBER).value)) + self.measure_offset
        self.expect(TokenType.BEAT) # :
        b_num = float(self.expect(TokenType.NUMBER).value)
        self.expect(TokenType.FRET, "Missing pipe separator '|' after beat number")
        
        events = []
        root_hint = None
        cue = ""
        
        while self.peek().type not in [TokenType.EOF, TokenType.MEASURE, TokenType.REPEAT, TokenType.RBRACE]:
            tok = self.peek()
            if tok.type == TokenType.STRING:
                s_name = self.advance().value
                self.expect(TokenType.BEAT) # :
                fret_val = str(self.expect(TokenType.NUMBER).value).replace('.0', '')
                events.append({'string': self.string_names.index(s_name), 'fret': fret_val})
            elif tok.type == TokenType.OVER:
                self.advance()
                # Root can be a STRING token (e.g. A, B, E) or an IDENTIFIER (e.g. F#m)
                hint_tok = self.advance()
                root_hint = str(hint_tok.value)
            elif tok.type == TokenType.CUE:
                cue = self.advance().value
            else:
                self.advance()

        if m_num not in self.measures: self.measures[m_num] = {}
        
        midi_pitches = [self.string_pitches[e['string']] + int(re.search(r'\d+', e['fret']).group()) for e in events]
        theory = MusicTheory.analyze_chord(midi_pitches, root_hint)
        
        self.measures[m_num][b_num] = {
            "events": events,
            "cue": cue,
            "theory": theory,
            "tip": ""
        }

    def render(self, measures_per_line=4, show_tips=True):
        output = []
        measure_numbers = sorted(self.measures.keys())
        for i in range(0, len(measure_numbers), measures_per_line):
            chunk_indices = measure_numbers[i:i + measures_per_line]
            output.append(f"\n--- MEASURES {chunk_indices[0]} - {chunk_indices[-1]} ({self.ts_num}/{self.ts_den}) ---")
            
            # Initialization with proper alignment for the vertical bar
            chunk_lines = [f"{n} |" for n in self.string_names]
            chunk_theory, chunk_cue, chunk_beats = "   ", "   ", "   "
            
            for m_num in chunk_indices:
                m_data = self.measures[m_num]
                
                # Dynamic step detection: Shrink step until all notes fit
                step = 1.0
                for b in m_data.keys():
                    frac = round(b % 1, 4)
                    if frac == 0: continue
                    while round(frac % step, 4) != 0 and step > 0.03125:
                        step /= 2
                
                num_slots = int(round(self.ts_num / step))
                beat_sequence = [round(j * step + 1, 4) for j in range(num_slots)]
                
                # Every measure starts with '--' padding
                m_lines = ["--" for _ in range(6)]
                m_theory, m_cue, m_beats = "  ", "  ", "  "
                last_theory, last_cue = "", ""
                
                for b in beat_sequence:
                    label = ""
                    if b.is_integer(): label = str(int(b))
                    elif round(b % 1, 2) == 0.5: label = "&"
                    elif round(b % 1, 2) == 0.25: label = "e"
                    elif round(b % 1, 2) == 0.75: label = "a"
                    
                    if b in m_data:
                        bd = m_data[b]
                        col = ["-"] * 6
                        for e in bd['events']: col[e['string']] = e['fret']
                        
                        # Compact Column Width: Ensure consistency with min 3 chars
                        col_w = max(3, len(label) + 1)
                        for e in bd['events']: col_w = max(col_w, len(e['fret']) + 1)
                        
                        for s in range(6): m_lines[s] += col[s].ljust(col_w, '-')
                        m_beats += label.ljust(col_w)
                        
                        # Float-over logic for theory/cues (with repeat suppression)
                        theory_to_add = bd['theory'] if bd['theory'] != last_theory else ""
                        cue_to_add = f"[{bd['cue']}]" if bd['cue'] and bd['cue'] != last_cue else ""
                        
                        m_theory += theory_to_add
                        m_cue += cue_to_add
                        
                        if theory_to_add: last_theory = bd['theory']
                        if cue_to_add: last_cue = bd['cue']
                        
                        target_len = len(m_lines[0])
                        m_theory = m_theory.ljust(target_len)
                        m_cue = m_cue.ljust(target_len)
                    else:
                        # Empty slot
                        fill_w = max(3, len(label) + 1)
                        for s in range(6): m_lines[s] += "-" * fill_w
                        m_beats += label.ljust(fill_w)
                        
                        target_len = len(m_lines[0])
                        m_theory = m_theory.ljust(target_len)
                        m_cue = m_cue.ljust(target_len)
                
                # Close the measure
                for s in range(6): chunk_lines[s] += m_lines[s] + "|"
                chunk_theory += m_theory + " "
                chunk_cue += m_cue + " "
                chunk_beats += m_beats + " "
            
            output.append(chunk_cue)
            output.append(chunk_theory)
            output.extend(chunk_lines)
            output.append(chunk_beats)
            
            # Collective Tips for the chunk
            if show_tips:
                for m_num in chunk_indices:
                    for b in self.measures[m_num]:
                        if self.measures[m_num][b].get('tip'):
                            output.append(f"   (M{m_num}) {self.measures[m_num][b]['tip']}")
                        
        return "\n".join(output)

if __name__ == "__main__":
    sample = """
    TS 6/4
    M1:1 | A:5 OVER D #[Intro]
    M1:5 | G:7
    M1:6 | G:6
    """
    comp = TabCompiler()
    comp.compile_text(sample)
    print(comp.render())
