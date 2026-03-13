import time
import os
import shutil
import sys
from tab_compiler import TabCompiler

DEFAULT_WATCH_FILE = "composition.tabbit"
DEFAULT_TS = (4, 4)

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def get_terminal_width():
    return shutil.get_terminal_size((80, 20)).columns

def process_chunk(chunk_lines, term_w):
    """
    Takes a list of lines representing one chunk (header + cues + strings + beats + theory).
    Returns a list of lines scrolled/windowed to fit term_w, synchronized by tab measure boundaries.
    """
    if not chunk_lines: return []
    
    # Identify the string lines (they start with e, B, G, D, A, or E and have '|')
    tab_line_indices = []
    for idx, line in enumerate(chunk_lines):
        if any(line.startswith(s + " |") for s in ['e', 'B', 'G', 'D', 'A', 'E']):
            tab_line_indices.append(idx)
    
    if not tab_line_indices:
        # If no tab lines, just return truncated
        return [line[:term_w] for line in chunk_lines]

    # Use the first tab line to determine scroll/measure boundaries
    master_line = chunk_lines[tab_line_indices[0]]
    if len(master_line) <= term_w:
        return chunk_lines

    # Find every '|' in the master tab string
    bar_indices = [i for i, char in enumerate(master_line) if char == '|']
    if len(bar_indices) < 2:
        return [line[:term_w] for line in chunk_lines]

    prefix_w = bar_indices[0] + 1
    available_w = term_w - prefix_w - 5 # -5 for "...|"
    
    # Work backwards from the right to see how many full measures fit
    # A measure exists between bar_indices[i] and bar_indices[i+1]
    fitted_indices = [] # stores (start, end) of measures to show
    current_w = 0
    for i in range(len(bar_indices) - 1, 0, -1):
        m_start = bar_indices[i-1] + 1
        m_end = bar_indices[i]
        m_len = m_end - m_start
        if current_w + m_len + 1 <= available_w:
            fitted_indices.insert(0, (m_start, m_end))
            current_w += m_len + 1
        else:
            break
            
    if not fitted_indices:
        return [f"... {line[-(term_w-4):]}" for line in chunk_lines]

    # Calculate global character start for the synchronized window
    window_start = fitted_indices[0][0]
    
    new_chunk = []
    for line in chunk_lines:
        prefix = line[:prefix_w]
        # Pad line if it's shorter than window_start (e.g. empty cue lines)
        full_line = line.ljust(window_start + current_w)
        
        if any(line.startswith(s + " |") for s in ['e', 'B', 'G', 'D', 'A', 'E']):
            # Tab String: [Name |] [...|] [Measure|Measure|]
            new_chunk.append(f"{prefix}...|{full_line[window_start:]}")
        else:
            # Meta Line: [Spaces] [... ] [Data       ]
            new_chunk.append(f"{prefix}... {full_line[window_start:]}")
            
    return new_chunk

def run_compiler(watch_file):
    if not os.path.exists(watch_file):
        with open(watch_file, "w") as f:
            f.write("# Write your tab shorthand here!\n")
            f.write("# Syntax: M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [E Major]\n")
            f.write("M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [Intro]\n")
        return

    # Look for a TS header
    current_ts = DEFAULT_TS
    with open(watch_file, "r") as f:
        lines = f.readlines()
        for line in lines:
            if line.startswith("TS "):
                try:
                    num, den = line.strip().split()[1].split('/')
                    current_ts = (int(num), int(den))
                except: pass

    compiler = TabCompiler(time_sig=current_ts)
    for line in lines:
        if line.strip() and not line.startswith("#") and not line.startswith("TS"):
            try:
                compiler.parse_line(line)
            except Exception as e:
                print(f"Error parsing line: {line}\n -> {e}")

    # Compile the full output
    full_output = compiler.compile(measures_per_line=4)
    term_w = get_terminal_width()
    
    # Process chunks separately for synchronized wrapping
    final_lines = []
    raw_chunks = full_output.split('\n\n---') # Split by measure group
    
    for i, raw_chunk in enumerate(raw_chunks):
        chunk_lines = raw_chunk.split('\n')
        # Re-add the header separator if it was split
        if i > 0: chunk_lines[0] = "---" + chunk_lines[0]
        
        final_lines.extend(process_chunk(chunk_lines, term_w))
        final_lines.append("") # Spacer between chunks

    clear_screen()
    print(f"🎸 Tabbit Live Composer | Watching: {watch_file} | TS: {current_ts[0]}/{current_ts[1]} | Width: {term_w}")
    print("-" * term_w)
    print("\n".join(final_lines))
    print("\n" + "=" * term_w)
    print("Waiting for changes... (Save your file to refresh)")

if __name__ == "__main__":
    # Get filename from command line or use default
    watch_file = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_WATCH_FILE
    
    last_mtime = 0
    while True:
        try:
            current_mtime = os.path.getmtime(watch_file) if os.path.exists(watch_file) else 0
            if current_mtime != last_mtime:
                run_compiler(watch_file)
                last_mtime = current_mtime
            time.sleep(0.5)
        except KeyboardInterrupt:
            print("\nExiting Live Composer.")
            break
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(2)
