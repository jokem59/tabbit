# Tabbit Project Context & Mandates

## Project Overview
Tabbit is a guitar tab toolkit featuring:
1.  **AI Transcriber (`tabbit.py`)**: Uses Spotify's `basic-pitch` and `demucs` to convert audio/YouTube to tabs.
2.  **Tab Transposer (`tab_parser.py`)**: Mathematically shifts ASCII tabs while preserving symbols and "string-hopping" for playability.
3.  **Live Composer (`tab_live.py` / `tab_compiler.py`)**: A real-time engine that renders shorthand notation (`M1:1 | E:0`) into formatted ASCII tabs with integrated music theory analysis.

## Core Mandates
- **Max Fret**: The transposer defaults to a max fret of **17** for playability, while the AI generator uses **24**.
- **Notation Standard**: Follow the syntax defined in `NOTATION.md`.
- **Sub-beat Logic & Greedy Resolution**: The compiler must dynamically adjust its grid resolution on a *per-measure* basis to the smallest detected subdivision (e.g., expanding to a 16th-note `1 e & a` grid only when `.25` or `.75` beats are present).
- **Formatting & Layout**: 
  - Every measure must begin with `--` padding so the first note falls on the second dash (`|--`).
  - Meta-layers (Cues, Chord Theory, Beats) must be stacked above and below the strings using clean whitespace formatting.
  - The `TabCompiler` must render up to **4 measures per line**.
- **Terminal Wrapping (Live Mode)**: If a chunk exceeds terminal width, `tab_live.py` must perform synchronized, measure-aware scrolling. All meta-layers and strings must move together as a single unit without breaking mid-measure.

## Current Workspace Context
- **Time Signature**: Currently focusing on **6/4** and **6/8** time.
- **Key Context**: Working on a riff originally in **Key of A (D chord context)**, often transposing **+3 semitones** to the **Key of C (F chord context)**.
- **Project Structure**:
    - `tab_compiler.py`: Logic for rendering shorthand (Lexer/Parser/Emitter).
    - `tab_live.py`: Terminal-aware file watcher for `.tabbit` files.
    - `tabbit_fmt.py`: Auto-formatter for shorthand syntax.
    - `NOTATION.md`: User-facing documentation for syntax.
    - `render_output.py`: Testing script for rendering shorthand.
