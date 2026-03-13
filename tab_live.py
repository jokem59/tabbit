import time
import os
from tab_compiler import TabCompiler

WATCH_FILE = "composition.tabbit"
DEFAULT_TS = (4, 4)

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def run_compiler():
    if not os.path.exists(WATCH_FILE):
        with open(WATCH_FILE, "w") as f:
            f.write("# Write your tab shorthand here!\n")
            f.write("# Syntax: M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [E Major]\n")
            f.write("M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [Intro]\n")
        return

    # Look for a TS header in the file
    current_ts = DEFAULT_TS
    with open(WATCH_FILE, "r") as f:
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

    clear_screen()
    print(f"🎸 Tabbit Live Composer | Watching: {WATCH_FILE} | TS: {current_ts[0]}/{current_ts[1]}")
    print("-" * 70)
    print(compiler.compile())
    print("\n" + "=" * 70)
    print("Waiting for changes... (Save your file to refresh)")

if __name__ == "__main__":
    last_mtime = 0
    while True:
        try:
            current_mtime = os.path.getmtime(WATCH_FILE) if os.path.exists(WATCH_FILE) else 0
            if current_mtime != last_mtime:
                run_compiler()
                last_mtime = current_mtime
            time.sleep(0.5)
        except KeyboardInterrupt:
            print("\nExiting Live Composer.")
            break
        except Exception as e:
            print(f"Error: {e}")
            time.sleep(2)
