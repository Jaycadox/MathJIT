use coloured_strings::*;

pub fn error_message(input: &str, start: usize, end: usize) -> String {
    let mut indic = String::with_capacity(input.len());
    indic.push_str(&input[..start]);
    let reg = &input[start..=end].to_string();
    indic.push_str(&colour(&reg, "red"));
    if input.len() > end {
        indic.push_str(&input[(end + 1)..]);
    }
    format!("\n{indic}")
}
