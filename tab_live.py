import time
import os
import shutil
import sys
from tab_compiler import TabCompiler
from tabbit_fmt import format_tabbit_file

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
    is_scrolled = window_start > prefix_w
    
    new_chunk = []
    for line in chunk_lines:
        prefix = line[:prefix_w]
        # Pad line if it's shorter than window_start (e.g. empty cue lines)
        full_line = line.ljust(window_start + current_w)
        
        scroll_marker = "|...|" if any(line.startswith(s + " |") for s in ['e', 'B', 'G', 'D', 'A', 'E']) else " ... "
        
        if is_scrolled:
            new_chunk.append(f"{prefix}{scroll_marker}{full_line[window_start:]}")
        else:
            new_chunk.append(full_line[:term_w])
            
    return new_chunk

def run_compiler(watch_file):
    if not os.path.exists(watch_file):
        with open(watch_file, "w") as f:
            f.write("# Write your tab shorthand here!\n")
            f.write("# Syntax: M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [E Major]\n")
            f.write("M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [Intro]\n")
        return

    with open(watch_file, "r") as f:
        content = f.read()

    compiler = TabCompiler()
    parsing_errors = []
    
    try:
        compiler.compile_text(content)
    except Exception as e:
        parsing_errors.append(f"❌ Compilation Error: {e}")
        import traceback
        parsing_errors.append(traceback.format_exc())

    # Compile the full output
    show_tips = "--no-tips" not in sys.argv
    full_output = compiler.render(measures_per_line=4, show_tips=show_tips)
    
    if not compiler.measures and not parsing_errors:
        parsing_errors.append("⚠️ No measures were detected in the file. Check your syntax (e.g., M1:1 | ...)")

    term_w = get_terminal_width()

    # If there were errors, show them at the bottom
    final_output = full_output
    if parsing_errors:
        final_output += "\n" + "-" * term_w + "\n"
        final_output += "\n".join(parsing_errors)

    clear_screen()
    print(f"🎸 Tabbit Live Composer | Watching: {watch_file} | TS: {compiler.ts_num}/{compiler.ts_den} | Width: {term_w}")
    if "--auto-fmt" in sys.argv: print("✨ Auto-Formatting: ON")
    if not show_tips: print("🔇 Tips: OFF")
    print("-" * term_w)
    print(final_output)
    print("\n" + "=" * term_w)
    print("Waiting for changes... (Save your file to refresh)")


if __name__ == "__main__":
    # Get filename from command line or use default, ignoring flags
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    watch_file = args[0] if args else DEFAULT_WATCH_FILE
    
    last_mtime = 0
    while True:
        try:
            current_mtime = os.path.getmtime(watch_file) if os.path.exists(watch_file) else 0
            if current_mtime != last_mtime:
                # Optional: Format the file before compiling
                if "--auto-fmt" in sys.argv:
                    format_tabbit_file(watch_file)
                    # Get NEW mtime after formatting to avoid double-triggering
                    current_mtime = os.path.getmtime(watch_file)
                
                run_compiler(watch_file)
                last_mtime = current_mtime
            time.sleep(0.5)
        except KeyboardInterrupt:
            print("\nExiting Live Composer.")
            break
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(2)
