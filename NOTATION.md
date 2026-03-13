# Tabbit Notation Guide

Tabbit supports two main types of notation: **Standard ASCII Tab** (for transposition and AI generation) and **Tabbit Shorthand** (for the Live Composer).

---

## 1. Standard ASCII Tab
Used by `tab_parser.py` (transposer) and output by `tabbit.py` (AI generator).

### Format
A standard 6-line block where each line represents a guitar string.

```text
e |--0---2h3p2---0--|
B |--1-----------1--|
G |--0-----------0--|
D |--2-----------2--|
A |--3-----------3--|
E |-----------------|
```

### Supported Symbols
*   **Numbers (`0-24`)**: Fret numbers.
*   **`h`**: Hammer-on (e.g., `5h7`)
*   **`p`**: Pull-off (e.g., `7p5`)
*   **`s` / `/` / `\`**: Slides
*   **`b`**: Bend
*   **`~` / `v`**: Vibrato
*   **`*`**: Harmonic
*   **`x2`, `X4`, etc.**: Repeat markers (ignored by the transposer to prevent accidental pitch shifts).

### Transposition Rules
The transposer (`tab_parser.py`) will automatically:
1.  Shift all fret numbers by the requested offset.
2.  **String Hop**: If a note becomes unplayable on its original string (fret < 0 or > 17), it will automatically move it to a different string if possible.
3.  **Marker Aware**: It preserves symbols like `~` or `h` during transposition.

---

## 2. Tabbit Shorthand (`.tabbit`)
Used by `tab_compiler.py` and `tab_live.py` for rapid composition.

### File Header
You can define the time signature at the top of the file:
```text
TS 4/4
```
(Defaults to 4/4 if not specified)

### Note Syntax
Each line represents a "beat" or "event" in a specific measure.

**Pattern:**  
`M{Measure}:{Beat} | {String}:{Fret} ... [OVER {Root}] [# [{Cue}]]`

*   **`M1:1`**: Measure 1, Beat 1.
*   **`M1:1.5`**: Measure 1, Beat 1.5 (the "and" of beat 1).
*   **`E:0 A:2 D:2`**: Play fret 0 on Low E, fret 2 on A, and fret 2 on D (an E5 power chord).
*   **`OVER G`**: (Optional) Triggers the music theory engine to analyze the chord relative to 'G' and stack the chord name (e.g., *G Major [1-3-5]*) above the tab. *Note: Chord analysis is only displayed if an OVER hint is provided.*
*   **`# [Intro]`**: (Optional) Adds a text cue/comment stacked above the tab.

### Sub-beats (Off-beats)
To play something between beats (e.g., "1 and 2 and"), use decimal increments:

*   **8th notes (`.5`)**: `1.5`, `2.5`... (Outputs as `1 & 2 &`)
*   **16th notes (`.25`)**: `1.25` (1-e), `1.5` (1-&), `1.75` (1-a).
*   **32nd notes (`.125`)**: `1.125`, `1.375`...

```text
M1:1    | E:0 # [Beat 1]
M1:1.25 | E:0 # [Beat 1 e]
M1:1.5  | E:0 # [Beat 1 &]
M1:1.75 | E:0 # [Beat 1 a]
```
**Greedy Resolution:** Tabbit uses smart spacing. If any sub-beat is detected in a measure, the compiler will automatically expand the output grid *for that measure only* to match its smallest subdivision, keeping simple measures compact.

### Examples

**A Simple Chord Progression:**
```text
TS 4/4
M1:1 | E:0 A:2 D:2 G:1 B:0 e:0 OVER E # [Verse Start]
M1:3 | E:0 A:2 D:2 G:1 B:0 e:0
M2:1 | A:3 D:5 G:5 B:5 OVER C # [C Major]
```

**Lead Licks:**
```text
M3:1 | G:7 B:8
M3:2 | G:9s11
M3:3 | e:12b14
```

### Live Composer Features
When using `tab_live.py`:
*   **Synchronized Scrolling**: If your tab gets wider than your terminal window, the view will automatically scroll horizontally. Cues, chords, strings, and beats all move as a single synchronized unit without breaking mid-measure.
*   **Real-time Analysis**: It identifies chords and detects inversions when `OVER` is used.
*   **Theory Tips**: Suggests embellishments (e.g., "Try a 'sus2' for a jangly feel") at the bottom of the screen.
*   **Auto-Refresh**: Save your `.tabbit` file, and the terminal view updates instantly.

---

## 3. How to Use Live Mode
To compose tabs in real-time with "Hot Reload" functionality:

1.  **Prepare a Shorthand File**: Create a file (e.g., `composition.tabbit` or `my_song.tabbit`).
2.  **Start the Watcher**: In your terminal, run the watcher and pass it your filename:
    ```bash
    python3 tab_live.py my_song.tabbit
    ```
    *(If no filename is provided, it defaults to `composition.tabbit`)*
3.  **Edit and Save**: Keep the terminal visible and open your `.tabbit` file in your code editor. Every time you **Save (Ctrl+S)**, the terminal will cleanly refresh with the rendered ASCII tab, aligned cues, and chord analysis.

**Recommended Setup**: Split your screen so your editor is on the left and the terminal running `tab_live.py` is on the right.

