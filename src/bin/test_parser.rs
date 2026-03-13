#[path = "../parser.rs"]
mod parser;

fn main() {
    let input = "M21:1    | D:7  OVER A  # [PM]";
    println!("Testing input: '{}'", input);
    let commands = parser::parse_tabbit(input);
    println!("Resulting commands: {:#?}", commands);
}
