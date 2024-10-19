use std::io;

#[derive(Debug)]
enum Error {
    Io(#[allow(dead_code)] io::Error),
    Parse,
}

impl From<io::Error> for Error {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}

fn main() -> Result<(), Error> {
    let path = std::env::args().nth(1).unwrap();
    let input = std::fs::read_to_string(path)?;
    let mut output = lang::parse(&input);
    // steal these errors so they don't get printed in the debug output
    let errs = std::mem::take(&mut output.errors);
    print!("{output:?}");
    if errs.is_empty() {
        Ok(())
    } else {
        for err in errs {
            eprintln!("{err:#?}");
        }
        Err(Error::Parse)
    }
}
