use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1, take_until},
    character::complete::{char, digit1, space0, space1, alphanumeric1, multispace0},
    combinator::{map, map_res, opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, pair, separated_pair, terminated},
    IResult,
};

#[derive(Debug, Clone)]
pub enum TabbitCommand {
    TimeSignature(u32, u32),
    Bpm(u32),
    MeasureEvent {
        measure: u32,
        beat: f32,
        notes: Vec<NoteEvent>,
        root_hint: Option<String>,
        cue: Option<String>,
    },
    Repeat {
        count: u32,
        commands: Vec<TabbitCommand>,
    },
    MacroDefine {
        name: String,
        commands: Vec<TabbitCommand>,
    },
    MacroUsage(String),
    SectionHeader(String),
    RootHint(String),
}

#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub string: String,
    pub fret: String,
}

fn parse_u32(input: &str) -> IResult<&str, u32> {
    map_res(digit1, |s: &str| s.parse::<u32>())(input)
}

fn parse_f32(input: &str) -> IResult<&str, f32> {
    map_res(
        recognize(pair(digit1, opt(pair(char('.'), digit1)))),
        |s: &str| s.parse::<f32>()
    )(input)
}

fn parse_ts(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, _) = tag("TS")(input)?;
    let (input, _) = space1(input)?;
    let (input, (num, den)) = separated_pair(parse_u32, char('/'), parse_u32)(input)?;
    Ok((input, TabbitCommand::TimeSignature(num, den)))
}

fn parse_note(input: &str) -> IResult<&str, NoteEvent> {
    let (input, string) = alt((
        tag("e"), tag("B"), tag("G"), tag("D"), tag("A"), tag("E")
    ))(input)?;
    let (input, _) = char(':')(input)?;
    let (input, fret) = take_while1(|c: char| c.is_digit(10) || "hpsbvr.".contains(c))(input)?;
    Ok((input, NoteEvent {
        string: string.to_string(),
        fret: fret.to_string(),
    }))
}

fn parse_cue_internal(input: &str) -> IResult<&str, String> {
    let (input, _) = char('#')(input)?;
    let (input, _) = space0(input)?;
    let (input, cue) = delimited(char('['), take_until("]"), char(']'))(input)?;
    Ok((input, cue.to_string()))
}

fn parse_section_header(input: &str) -> IResult<&str, TabbitCommand> {
    map(parse_cue_internal, TabbitCommand::SectionHeader)(input)
}

fn parse_root_hint(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, _) = tag("OVER")(input)?;
    let (input, _) = space1(input)?;
    let (input, root) = take_while1(|c: char| c.is_alphanumeric() || c == '#')(input)?;
    Ok((input, TabbitCommand::RootHint(root.to_string())))
}

fn parse_measure_event(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, _) = char('M')(input)?;
    let (input, measure) = parse_u32(input)?;
    let (input, _) = char(':')(input)?;
    let (input, beat) = parse_f32(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('|')(input)?;
    let (input, _) = space0(input)?;
    
    let (input, notes) = many1(terminated(parse_note, space0))(input)?;
    
    Ok((input, TabbitCommand::MeasureEvent {
        measure,
        beat,
        notes,
        root_hint: None,
        cue: None,
    }))
}

fn parse_repeat(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, _) = tag("REPEAT")(input)?;
    let (input, _) = space1(input)?;
    let (input, count) = parse_u32(input)?;
    let (input, _) = space0(input)?;
    let (input, commands) = delimited(
        pair(char('{'), multispace0),
        many0(terminated(parse_command, multispace0)),
        char('}')
    )(input)?;
    
    Ok((input, TabbitCommand::Repeat { count, commands }))
}

fn parse_macro_define(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, name) = recognize(pair(alphanumeric1, many0(alt((alphanumeric1, tag("_"))))))(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = space0(input)?;
    let (input, commands) = delimited(
        pair(char('{'), multispace0),
        many0(terminated(parse_command, multispace0)),
        char('}')
    )(input)?;
    
    Ok((input, TabbitCommand::MacroDefine {
        name: name.to_string(),
        commands,
    }))
}

fn parse_comment(input: &str) -> IResult<&str, ()> {
    let (input, _) = char('#')(input)?;
    let (input, _) = take_while1(|c| c != '\n')(input)?;
    Ok((input, ()))
}

fn parse_bpm(input: &str) -> IResult<&str, TabbitCommand> {
    let (input, _) = tag("BPM")(input)?;
    let (input, _) = space1(input)?;
    let (input, bpm) = parse_u32(input)?;
    Ok((input, TabbitCommand::Bpm(bpm)))
}

fn parse_command(input: &str) -> IResult<&str, TabbitCommand> {
    alt((
        parse_ts,
        parse_bpm,
        parse_measure_event,
        parse_repeat,
        parse_macro_define,
        parse_section_header,
        parse_root_hint,
        map(recognize(pair(alphanumeric1, many0(alt((alphanumeric1, tag("_")))))), |s: &str| TabbitCommand::MacroUsage(s.to_string()))
    ))(input)
}

pub fn parse_tabbit(input: &str) -> Vec<TabbitCommand> {
    let mut results = Vec::new();
    let mut lines = input.lines();
    
    while let Some(line) = lines.next() {
        let mut curr = line.trim();
        if curr.is_empty() { continue; }
        
        if curr.starts_with('#') && !curr.starts_with("# [") {
            continue;
        }
        
        let mut line_measure_event_idx: Option<usize> = None;

        while !curr.is_empty() {
            curr = curr.trim_start();
            if curr.is_empty() { break; }
            
            match parse_command(curr) {
                Ok((rest, cmd)) => {
                    match cmd {
                        TabbitCommand::RootHint(hint) => {
                            if let Some(idx) = line_measure_event_idx {
                                if let TabbitCommand::MeasureEvent { root_hint, .. } = &mut results[idx] {
                                    *root_hint = Some(hint);
                                }
                            }
                        }
                        TabbitCommand::SectionHeader(cue) => {
                            if let Some(idx) = line_measure_event_idx {
                                if let TabbitCommand::MeasureEvent { cue: existing_cue, .. } = &mut results[idx] {
                                    *existing_cue = Some(cue);
                                }
                            } else {
                                results.push(TabbitCommand::SectionHeader(cue));
                            }
                        }
                        TabbitCommand::MeasureEvent { .. } => {
                            line_measure_event_idx = Some(results.len());
                            results.push(cmd);
                        }
                        _ => results.push(cmd),
                    }
                    curr = rest;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
    results
}
