pub fn parse(input: &str) -> Vec<String> {
    input.lines().map(|l| l.to_string()).collect()
}
