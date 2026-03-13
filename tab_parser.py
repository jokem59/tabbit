import sys
import re

print("🎸 Org-Mode Tab Transposer (Max Fret: 17, Marker Aware)\n" + "-"*55)

try:
    offset = int(input("Transpose by how many half-steps? (e.g., -2 or +3): "))
except ValueError:
    print("Invalid number. Exiting.")
    sys.exit(1)

print("\nPaste your tabs below. When finished, press Ctrl+D (Mac/Linux) or Ctrl+Z then Enter (Windows) on a new line:\n")
raw_text = sys.stdin.read()

lines = raw_text.splitlines()

# Standard tuning MIDI pitches (High e down to Low E)
string_pitches = [64, 59, 55, 50, 45, 40]

# 1. Identify 6-line tab blocks and ignore headers/rhythm markers
blocks = []
current_block = []
for i, line in enumerate(lines):
    if re.match(r'^([eBGDAbgdaE])\s*\|', line):
        current_block.append(i)
    else:
        if len(current_block) == 6:
            blocks.append(current_block)
        current_block = []
if len(current_block) == 6:
    blocks.append(current_block)

# 2. Process each block mathematically
for block_indices in blocks:
    canvas = []
    notes = []
    
    # Initialize the canvas by copying the original lines exactly
    for line_idx in block_indices:
        canvas.append(list(lines[line_idx]))
    
    # Extract notes + symbols and erase them from the canvas
    for s_idx, line_idx in enumerate(block_indices):
        line = lines[line_idx]
        
        # REGEX UPDATE: 
        # (?<![xX]) ensures we ignore repeat markers like x2 or X4
        # ([~shpb/\\*v]?) explicitly only captures valid guitar symbols as baggage
        for match in re.finditer(r'(?<![xX])(\d+)([~shpb/\\*v]?)', line):
            start = match.start()
            fret = int(match.group(1))
            symbol = match.group(2) if match.group(2) else ""
            end = start + len(match.group(0)) 
            
            notes.append((s_idx, start, fret, symbol))
            
            # Erase the entire note AND symbol from the canvas, leaving clean hyphens
            for i in range(start, end):
                if i < len(canvas[s_idx]):
                    canvas[s_idx][i] = '-'
                    
    # Calculate new pitches and stamp them onto the blanked canvas
    for s_idx, start, old_fret, symbol in notes:
        old_pitch = string_pitches[s_idx] + old_fret
        new_pitch = old_pitch + offset
        
        new_s_idx = s_idx
        new_fret = new_pitch - string_pitches[s_idx]
        
        # STRING HOPPING LOGIC: If unplayable or > 17, find a new string
        if new_fret > 17 or new_fret < 0:
            found = False
            for i in range(6):
                test_fret = new_pitch - string_pitches[i]
                if 0 <= test_fret <= 17:
                    new_s_idx = i
                    new_fret = test_fret
                    found = True
                    break
            if not found:
                new_fret = "X"
                
        # Build the new stamp (e.g., "11~" instead of just "11")
        stamp = str(new_fret) + symbol if new_fret != "X" else "X" + symbol
        
        # Stamp it onto the canvas at the exact original horizontal position
        for i, char in enumerate(stamp):
            write_pos = start + i
            if write_pos < len(canvas[new_s_idx]):
                canvas[new_s_idx][write_pos] = char
                
    # Reassemble the text block
    for s_idx, line_idx in enumerate(block_indices):
        lines[line_idx] = "".join(canvas[s_idx])

# 3. Print the final result
print("\n\n[ Transposed Output ]\n" + "="*45 + "\n")
print("\n".join(lines))
