//! Configuration as `key = value` lines, readable and writable by a person. A damaged
//! line loses its own setting and not the others; a line without `=` is ignored, which
//! makes room for comments.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfText(Vec<(String, String)>);

impl ConfText {
    pub fn parse(text: &str) -> Self {
        Self(
            text.lines()
                .filter_map(|line| line.split_once('='))
                .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
                .collect(),
        )
    }

    /// The last value given for `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    pub fn render(pairs: &[(&str, &str)]) -> String {
        pairs.iter().map(|(key, value)| format!("{key} = {value}\n")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_values_are_trimmed_and_the_rest_ignored() {
        let conf = ConfText::parse("# where\nplace =  Paris \nlatitude=48.85\nnonsense\n");
        assert_eq!(conf.get("place"), Some("Paris"));
        assert_eq!(conf.get("latitude"), Some("48.85"));
        assert_eq!(conf.get("nonsense"), None);
    }

    #[test]
    fn a_value_may_hold_an_equals_sign() {
        assert_eq!(ConfText::parse("password = a=b=c").get("password"), Some("a=b=c"));
    }

    #[test]
    fn what_is_rendered_parses_back() {
        let text = ConfText::render(&[("backlight", "10s"), ("reading_lamp", "off")]);
        assert_eq!(text, "backlight = 10s\nreading_lamp = off\n");
        assert_eq!(ConfText::parse(&text).get("reading_lamp"), Some("off"));
    }
}
