# Tabbit Project Context & Mandates

## Project Overview
Tabbit is a guitar tab toolkit featuring:
1.  **AI Transcriber (`tabbit.py`)**: Uses Spotify's `basic-pitch` and `demucs` to convert audio/YouTube to tabs.
2.  **Tab Transposer (`tab_parser.py`)**: Mathematically shifts ASCII tabs while preserving symbols and "string-hopping" for playability.
3.  **Live Composer (`tab_live.py` / `tab_compiler.py`)**: A real-time engine that renders shorthand notation (`M1:1 | E:0`) into formatted ASCII tabs with integrated music theory analysis.

## Core Mandates
- **Max Fret**: The transposer defaults to a max fret of **17** for playability, while the AI generator uses **24**.
- **Notation Standard**: Follow the syntax defined in `NOTATION.md`.
- **Sub-beat Logic**: Support fractional beats (e.g., `1.5` for 8th notes, `1.25` for 16ths). The compiler must dynamically adjust its grid resolution to the smallest detected subdivision.
- **Formatting**: The `TabCompiler` must render **4 measures per line** using `|` as a measure boundary.

## Current Workspace Context
- **Time Signature**: Currently focusing on **6/4** time.
- **Key Context**: Working on a riff originally in **Key of A (D chord context)**, often transposing **+3 semitones** to the **Key of C (F chord context)**.
- **Project Structure**:
    - `tab_compiler.py`: Logic for rendering shorthand.
    - `tab_live.py`: File watcher for `composition.tabbit`.
    - `NOTATION.md`: User-facing documentation for syntax.
    - `render_output.py`: Testing script for rendering shorthand.

## Active Riff (Shorthand Reference)
```text
M1:1 | A:5 OVER D # [Intro]
M1:5 | G:7
M1:6 | G:6
M2:1 | G:7
M2:5 | G:7
...
M6:5 | A:5h7
M6:6 | D:4s5
```
