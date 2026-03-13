import re
import sys
import os

def format_tabbit_file(file_path):
    if not os.path.exists(file_path):
        return

    with open(file_path, 'r') as f:
        lines = f.readlines()

    # Regex matching the Tabbit shorthand pattern
    pattern = r"^(M\d+:\d+\.?\d*)\s*\|\s*(.*?)(?=\s+[Oo][Vv][Ee][Rr]\s+|\s*#|$)(?:\s+[Oo][Vv][Ee][Rr]\s+(\S+))?(?:\s*#\s*\[(.*)\])?$"
    
    # Pass 1: Group lines into sections
    sections = []
    current_section = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("# ["):
            if current_section:
                sections.append(current_section)
            current_section = [line]
        else:
            current_section.append(line)
    if current_section:
        sections.append(current_section)

    # Pass 2: Format each section independently
    new_content_lines = []
    for section_lines in sections:
        max_mb = 0
        max_notes_width = 0
        max_over_width = 0
        max_fret_widths = {} # string -> max_len of fret part
        
        # Pass 2a: Analyze section for max widths
        for line in section_lines:
            match = re.match(pattern, line.strip())
            if match:
                max_mb = max(max_mb, len(match.group(1)))
                
                # Internal note alignment: track max fret width per string
                tokens = match.group(2).strip().split()
                for t in tokens:
                    if ":" in t:
                        s, f = t.split(":", 1)
                        max_fret_widths[s] = max(max_fret_widths.get(s, 0), len(f))
                
                if match.group(3):
                    max_over_width = max(max_over_width, len(match.group(3)))

        # Pass 2b: Calculate max width of the reassembled notes column
        for line in section_lines:
            match = re.match(pattern, line.strip())
            if match:
                tokens = match.group(2).strip().split()
                reassembled_len = 0
                for i, t in enumerate(tokens):
                    if ":" in t:
                        s, f = t.split(":", 1)
                        # Length is string name + ":" + padded fret
                        reassembled_len += len(s) + 1 + max_fret_widths[s]
                    else:
                        reassembled_len += len(t)
                    if i < len(tokens) - 1:
                        reassembled_len += 1 # Space
                max_notes_width = max(max_notes_width, reassembled_len)

        # Pass 2c: Reconstruct lines for this section
        for line in section_lines:
            stripped = line.strip()
            match = re.match(pattern, stripped)
            
            if not match:
                new_content_lines.append(line.rstrip())
                continue
                
            mb = match.group(1)
            notes_raw = match.group(2).strip().split()
            over = match.group(3) if match.group(3) else ""
            cue = match.group(4) if match.group(4) else ""

            # Column 1: Measure:Beat
            row = mb.ljust(max_mb) + " | "
            
            # Column 2: Notes (internally padded)
            new_note_tokens = []
            for t in notes_raw:
                if ":" in t:
                    s, f = t.split(":", 1)
                    new_note_tokens.append(f"{s}:{f.ljust(max_fret_widths[s])}")
                else:
                    new_note_tokens.append(t)
            
            notes_str = " ".join(new_note_tokens)
            row += notes_str.ljust(max_notes_width)
            
            # Column 3: OVER Hint
            if max_over_width > 0:
                if over:
                    row += (" OVER " + over).ljust(max_over_width + 6)
                else:
                    row += " " * (max_over_width + 6)
                    
            # Column 4: Cue
            if cue:
                row += " # [" + cue + "]"
                
            new_content_lines.append(row.rstrip())

    # Only write if content actually changed
    new_content = "\n".join(new_content_lines) + "\n"
    with open(file_path, 'r') as f:
        old_content = f.read()
    
    if new_content != old_content:
        with open(file_path, 'w') as f:
            f.write(new_content)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 tabbit_fmt.py <file.tabbit>")
    else:
        format_tabbit_file(sys.argv[1])
