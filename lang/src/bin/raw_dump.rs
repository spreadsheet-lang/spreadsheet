use std::io;

fn main() -> Result<(), io::Error> {
    let path = std::env::args().nth(1).unwrap();
    let input = std::fs::read_to_string(path)?;
    let output = lang::parse(&input);
    eprintln!("{output:#?}");
    Ok(())
}
