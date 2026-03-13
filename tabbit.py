import os
import random
import subprocess
import yt_dlp
from basic_pitch.inference import predict

class GuitarTabEngine:
    def __init__(self):
        # Standard tuning MIDI pitches: e (64), B (59), G (55), D (50), A (45), E (40)
        self.string_pitches = [64, 59, 55, 50, 45, 40]
        self.string_names = ['e', 'B', 'G', 'D', 'A', 'E']
        self.max_fret = 24

    def calculate_best_position(self, pitch, forced_string=None):
        """Finds the logical string and fret for a given pitch."""
        if forced_string is not None:
            fret = pitch - self.string_pitches[forced_string]
            if 0 <= fret <= self.max_fret:
                return forced_string, fret
            return None, None 

        # Default heuristic: Try to play it between frets 0 and 12, preferring higher strings
        for s, string_base in enumerate(self.string_pitches):
            fret = pitch - string_base
            if 0 <= fret <= 12:
                return s, fret
                
        # Fallback: allow up to max_fret
        for s, string_base in enumerate(self.string_pitches):
            fret = pitch - string_base
            if 0 <= fret <= self.max_fret:
                return s, fret
                
        return None, None

    def shuffle_voicings(self, notes_data, key_offset=0):
        """Randomly assigns notes to different valid strings to generate alternate voicings."""
        for note in notes_data:
            target_pitch = note['pitch'] + key_offset
            possible_voicings = []
            
            for s, string_base in enumerate(self.string_pitches):
                fret = target_pitch - string_base
                if 0 <= fret <= self.max_fret:
                    possible_voicings.append(s)
            
            if possible_voicings:
                note['forced_string'] = random.choice(possible_voicings)
        return notes_data

    def generate_plaintext_tab(self, notes_data, key_offset=0, max_width=80):
        """Renders the list of notes into formatted ASCII guitar tabs, broken into measures."""
        tab_data = [f"{name}|-" for name in self.string_names]
        
        # Group notes by timestamp (groups notes played within 50ms into chords)
        grouped_notes = []
        for note in notes_data:
            if not grouped_notes:
                grouped_notes.append([note])
                continue
                
            last_group = grouped_notes[-1]
            if abs(note['time'] - last_group[0]['time']) < 0.05:
                last_group.append(note)
            else:
                grouped_notes.append([note])
        
        # Build the continuous tab lines
        for group in grouped_notes:
            column = ["-"] * 6 
            
            for note in group:
                adjusted_pitch = note['pitch'] + key_offset
                forced_string = note.get('forced_string')
                
                string_idx, fret = self.calculate_best_position(adjusted_pitch, forced_string)
                
                if string_idx is not None:
                    column[string_idx] = str(fret)
                    
            # Pad the column to ensure multi-digit frets don't break vertical alignment
            max_len = max(len(val) for val in column)
            for i in range(6):
                padding = "-" * ((max_len - len(column[i])) + 1)
                tab_data[i] += column[i] + padding + "-"
        
        # Chunk the output into readable blocks for the terminal
        final_output = []
        total_length = len(tab_data[0])
        prefix_length = 3 # "e|-" is 3 characters
        chunk_size = max_width - prefix_length
        
        for i in range(prefix_length, total_length, chunk_size):
            chunk_block = []
            for j in range(6):
                prefix = f"{self.string_names[j]}|-"
                chunk = tab_data[j][i:i + chunk_size]
                chunk_block.append(prefix + chunk)
            
            final_output.append("\n".join(chunk_block) + "\n")
            
        return "\n".join(final_output)


def download_youtube_audio(url, start_time=None, end_time=None):
    """Downloads audio from YouTube using yt-dlp."""
    print(f"\n[1/4] Downloading audio from {url}...")
    
    ydl_opts = {
        'format': 'bestaudio/best',
        'postprocessors': [{
            'key': 'FFmpegExtractAudio',
            'preferredcodec': 'wav',
            'preferredquality': '192',
        }],
        'outtmpl': 'temp_audio.%(ext)s',
        'quiet': True,
        'no_warnings': True
    }
    
    if start_time is not None and end_time is not None:
        ydl_opts['postprocessor_args'] = [
            '-ss', str(start_time),
            '-to', str(end_time)
        ]

    try:
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            ydl.download([url])
        print("Download complete.")
        return "temp_audio.wav"
    except Exception as e:
        print(f"Failed to download: {e}")
        return None


def isolate_guitar(audio_file_path):
    """Uses Demucs to separate the audio and returns the path to the 'other' stem (guitar)."""
    print(f"\n[2/4] Isolating the guitar track with Demucs... (This will take a while!)")
    
    try:
        # Run demucs via command line using subprocess
        subprocess.run(["demucs", audio_file_path], check=True, capture_output=False)
        
        # Demucs automatically creates a folder structure: separated/htdemucs/{filename}/
        base_name = os.path.splitext(os.path.basename(audio_file_path))[0]
        guitar_stem_path = os.path.join("separated", "htdemucs", base_name, "other.wav")
        
        if os.path.exists(guitar_stem_path):
            print(f"Guitar isolation complete! Found at: {guitar_stem_path}")
            return guitar_stem_path
        else:
            print("Error: Could not find the separated guitar track.")
            return audio_file_path
            
    except Exception as e:
        print(f"Demucs failed: {e}. Falling back to original audio.")
        return audio_file_path


def transcribe_audio_to_midi(audio_file_path, min_duration_sec=0.08):
    """Analyzes the audio file using Spotify's basic-pitch ML model and applies a time-based noise gate."""
    print(f"\n[3/4] Analyzing pitches with AI (Noise Gate active)...")
    
    try:
        _, _, note_events = predict(audio_file_path)
    except Exception as e:
        print(f"Error running basic-pitch: {e}")
        return []

    notes = []
    
    # note_events is a list of tuples: (start_time, end_time, pitch_midi, amplitude, [pitch_bends])
    for event in note_events:
        start_time, end_time, pitch_midi, amplitude = event[:4]
        
        # Calculate exactly how long the note was held
        duration = end_time - start_time
        
        # NOISE GATE: Filter out quiet background noise AND notes that are too short
        if amplitude > 0.2 and duration >= min_duration_sec: 
            notes.append({
                'pitch': int(round(pitch_midi)),
                'time': start_time,
                'duration': duration # Storing this just in case we need it later
            })
            
    # Sort chronologically
    notes.sort(key=lambda x: x['time'])
    print(f"Successfully extracted {len(notes)} valid notes (Filtered out transients shorter than {min_duration_sec}s)!")
    return notes


if __name__ == "__main__":
    print("🎸 TabGenius AI Backend (with Demucs Isolation)\n" + "-"*45)
    
    yt_url = input("Enter a YouTube URL: ")
    if not yt_url:
        print("URL required to run the ML model. Exiting.")
        exit()

    do_crop = input("Do you want to specify start/stop times? (y/n): ").lower()
    start_sec, end_sec = None, None
    if do_crop == 'y':
        start_sec = input("Start time (in seconds): ")
        end_sec = input("End time (in seconds): ")

    # 1. Download
    raw_audio_file = download_youtube_audio(yt_url, start_sec, end_sec)
    if not raw_audio_file:
        exit()
        
    # 2. Isolate Guitar (NEW STEP)
    isolated_guitar_file = isolate_guitar(raw_audio_file)
    
    # 3. Extract Pitches (Now using only the isolated guitar track)
    notes = transcribe_audio_to_midi(isolated_guitar_file)
    
    if not notes:
        print("No notes were detected, or an error occurred.")
    else:
        # 4. Generate Tabs
        print("\n[4/4] Generating Tabs...")
        engine = GuitarTabEngine()
        
        print("\n--- Original Key ---")
        print(engine.generate_plaintext_tab(notes, key_offset=0))
        
        print("\n--- Transposed Down 2 Half-Steps (Whole Step) ---")
        print(engine.generate_plaintext_tab(notes, key_offset=-2))
        
        print("\n--- Alternate Voicing (Original Key, played on different strings) ---")
        shuffled_notes = engine.shuffle_voicings(notes, key_offset=0)
        print(engine.generate_plaintext_tab(shuffled_notes, key_offset=0))
    
    # Optional Cleanup: Remove the original temp download
    if os.path.exists("temp_audio.wav"):
        os.remove("temp_audio.wav")
