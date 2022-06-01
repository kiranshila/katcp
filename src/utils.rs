/// Unescapes a string revieved from katcp using the eight valid escape characters
pub fn unescape(input: &str) -> String {
    input
        .replace(r"\_", " ")
        .replace(r"\0", "\0")
        .replace(r"\n", "\n")
        .replace(r"\r", "\r")
        .replace(r"\e", "\u{001B}")
        .replace(r"\t", "\t")
        .replace(r"\@", "")
        .replace(r"\\", r"\")
}

/// Escapes a string into a string suitable for katcp using the eight valid escape characters
pub fn escape(input: &str) -> String {
    if input.is_empty() {
        r"\@".to_owned()
    } else {
        input
            .replace('\\', r"\\")
            .replace(' ', r"\_")
            .replace('\0', r"\0")
            .replace('\n', r"\n")
            .replace('\r', r"\r")
            .replace('\u{001B}', r"\e")
            .replace('\t', r"\t")
    }
}

#[cfg(test)]
mod strings {
    use super::*;

    #[test]
    fn test_escape() {
        assert_eq!(r"This\_is\_my\_foo\n", escape("This is my foo\n"));
        assert_eq!("This is my foo\n", unescape(r"This\_is\_my\_foo\n"));
    }
}
