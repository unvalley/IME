//! Incremental romaji-to-hiragana composition.

use std::fmt;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RomajiComposer {
    pending: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidRomaji(pub char);

impl fmt::Display for InvalidRomaji {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported romaji character: {:?}", self.0)
    }
}

impl std::error::Error for InvalidRomaji {}

impl RomajiComposer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: String::new(),
        }
    }

    /// Adds one ASCII letter and returns any hiragana that became unambiguous.
    ///
    /// The uncommitted suffix remains available through [`Self::pending`].
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRomaji`] when `character` is not an ASCII letter or an
    /// apostrophe. The pending composition is not changed in that case.
    pub fn push(&mut self, character: char) -> Result<String, InvalidRomaji> {
        if !character.is_ascii_alphabetic() && character != '\'' {
            return Err(InvalidRomaji(character));
        }

        self.pending.push(character.to_ascii_lowercase());
        Ok(self.resolve(false))
    }

    #[must_use]
    pub fn pending(&self) -> &str {
        &self.pending
    }

    /// Removes one uncommitted romaji character.
    pub fn backspace(&mut self) -> bool {
        self.pending.pop().is_some()
    }

    /// Resolves a trailing `n` and returns other incomplete input literally.
    pub fn flush(&mut self) -> String {
        let mut output = self.resolve(true);
        output.push_str(&self.pending);
        self.pending.clear();
        output
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    fn resolve(&mut self, flush: bool) -> String {
        let mut output = String::new();

        loop {
            if self.pending.is_empty() {
                break;
            }

            let bytes = self.pending.as_bytes();
            if bytes.len() >= 2 {
                let first = bytes[0];
                let second = bytes[1];

                if first == second && is_consonant(first) && first != b'n' {
                    output.push('っ');
                    self.pending.remove(0);
                    continue;
                }

                if first == b'n' {
                    if second == b'\'' {
                        output.push('ん');
                        self.pending.drain(..2);
                        continue;
                    }
                    if second == b'n' || (!is_vowel(second) && second != b'y') {
                        output.push('ん');
                        self.pending.remove(0);
                        continue;
                    }
                }
            }

            if let Some(kana) = exact_match(&self.pending)
                && (flush || !has_longer_match(&self.pending))
            {
                output.push_str(kana);
                self.pending.clear();
                continue;
            }

            if has_prefix(&self.pending) || self.pending.len() == 1 {
                break;
            }

            // Preserve unsupported but valid ASCII sequences instead of losing input.
            output.push(self.pending.remove(0));
        }

        output
    }
}

const fn is_vowel(byte: u8) -> bool {
    matches!(byte, b'a' | b'i' | b'u' | b'e' | b'o')
}

const fn is_consonant(byte: u8) -> bool {
    byte.is_ascii_alphabetic() && !is_vowel(byte)
}

fn exact_match(input: &str) -> Option<&'static str> {
    ROMAJI_TABLE
        .iter()
        .find_map(|(romaji, kana)| (*romaji == input).then_some(*kana))
}

fn has_prefix(input: &str) -> bool {
    ROMAJI_TABLE
        .iter()
        .any(|(romaji, _)| romaji.starts_with(input))
}

fn has_longer_match(input: &str) -> bool {
    ROMAJI_TABLE
        .iter()
        .any(|(romaji, _)| romaji.len() > input.len() && romaji.starts_with(input))
}

const ROMAJI_TABLE: &[(&str, &str)] = &[
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sa", "さ"),
    ("si", "し"),
    ("shi", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("sho", "しょ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("ji", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("zya", "じゃ"),
    ("zyu", "じゅ"),
    ("zyo", "じょ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("jo", "じょ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("chi", "ち"),
    ("tu", "つ"),
    ("tsu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("cho", "ちょ"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("dya", "ぢゃ"),
    ("dyu", "ぢゅ"),
    ("dyo", "ぢょ"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("n", "ん"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("fu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("wa", "わ"),
    ("wo", "を"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("xtu", "っ"),
    ("ltu", "っ"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
];

#[cfg(test)]
mod tests {
    use super::RomajiComposer;

    fn compose(input: &str) -> String {
        let mut composer = RomajiComposer::new();
        let mut output = String::new();
        for character in input.chars() {
            output.push_str(&composer.push(character).unwrap());
        }
        output.push_str(&composer.flush());
        output
    }

    #[test]
    fn converts_basic_syllables() {
        assert_eq!(compose("nihongo"), "にほんご");
        assert_eq!(compose("watashi"), "わたし");
    }

    #[test]
    fn converts_contracted_sounds() {
        assert_eq!(compose("kyoushitsu"), "きょうしつ");
        assert_eq!(compose("ryokou"), "りょこう");
    }

    #[test]
    fn converts_double_consonant() {
        assert_eq!(compose("kitte"), "きって");
        assert_eq!(compose("gakkou"), "がっこう");
    }

    #[test]
    fn handles_syllabic_n() {
        assert_eq!(compose("konna"), "こんな");
        assert_eq!(compose("kanpai"), "かんぱい");
        assert_eq!(compose("kin'youbi"), "きんようび");
        assert_eq!(compose("hon"), "ほん");
    }

    #[test]
    fn retains_ambiguous_suffix_until_resolved() {
        let mut composer = RomajiComposer::new();
        assert_eq!(composer.push('s').unwrap(), "");
        assert_eq!(composer.push('h').unwrap(), "");
        assert_eq!(composer.pending(), "sh");
        assert_eq!(composer.push('i').unwrap(), "し");
        assert_eq!(composer.pending(), "");
    }

    #[test]
    fn backspace_edits_pending_input() {
        let mut composer = RomajiComposer::new();
        composer.push('k').unwrap();
        assert!(composer.backspace());
        assert_eq!(composer.pending(), "");
        assert!(!composer.backspace());
    }

    #[test]
    fn rejects_non_romaji_characters_without_mutation() {
        let mut composer = RomajiComposer::new();
        assert!(composer.push('1').is_err());
        assert_eq!(composer.pending(), "");
    }
}
