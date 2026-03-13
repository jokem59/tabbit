from tab_compiler import TabCompiler

compiler = TabCompiler(time_sig=(4, 4))

# Let's fix the shorthand to properly use OVER
shorthand = [
    "M1:1 | E:2 A:0 D:0 G:2 B:3 OVER D # [D/F# Chord]",
    "M1:3 | G:2h4 B:3 OVER G # [Lead line over G]"
]

for line in shorthand:
    compiler.parse_line(line)
    
print(compiler.compile())
