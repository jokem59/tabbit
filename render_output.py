from tab_compiler import TabCompiler

# Initialize compiler for 6/4 time
compiler = TabCompiler(time_sig=(6, 4))

# Your shorthand notation (Original Key of A / D Chord context)
shorthand_input = [
    "TS 6/4",
    "M1:1 | A:5 OVER D # [Intro]",
    "M1:5 | G:7",
    "M1:6 | G:6",
    "M2:1 | G:7",
    "M2:5 | G:7",
    "M3:1 | G:6",
    "M3:5 | G:6h7",
    "M4:1 | G:7",
    "M4:5 | B:8",
    "M5:1 | B:7",
    "M6:5 | A:5h7",
    "M6:6 | D:4s5",
    "M7:1 | A:5",
    "M7:5 | G:7",
    "M7:6 | G:6"
]

for line in shorthand_input:
    if line.strip() and not line.startswith("#") and not line.startswith("TS"):
        compiler.parse_line(line)

print(compiler.compile())
