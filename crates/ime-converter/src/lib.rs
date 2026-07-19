//! A small, deterministic kana-kanji conversion baseline.
//!
//! The current dictionary is intentionally tiny. The API and tests establish the
//! lattice/candidate behavior before a compiled dictionary format is introduced.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DictionaryEntry {
    pub reading: &'static str,
    pub surface: &'static str,
    pub word_cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub surface: String,
    pub cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Segment {
    pub reading: String,
    pub surface: String,
    pub cost: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversion {
    pub surface: String,
    pub segments: Vec<Segment>,
    pub cost: i32,
}

#[derive(Clone, Debug)]
pub struct Dictionary {
    entries: Vec<DictionaryEntry>,
}

impl Dictionary {
    #[must_use]
    pub fn new(mut entries: Vec<DictionaryEntry>) -> Self {
        entries.sort_unstable_by_key(|entry| (entry.reading, entry.word_cost, entry.surface));
        Self { entries }
    }

    #[must_use]
    pub fn bundled() -> Self {
        Self::new(BUNDLED_ENTRIES.to_vec())
    }

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn candidates(&self, reading: &str) -> Vec<Candidate> {
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.reading == reading)
            .map(|entry| Candidate {
                surface: entry.surface.to_owned(),
                cost: entry.word_cost,
            })
            .collect();

        if let Some(best) = self.convert_best(reading)
            && !candidates
                .iter()
                .any(|candidate| candidate.surface == best.surface)
        {
            candidates.push(Candidate {
                surface: best.surface,
                cost: best.cost,
            });
        }

        if !candidates
            .iter()
            .any(|candidate| candidate.surface == reading)
        {
            candidates.push(Candidate {
                surface: reading.to_owned(),
                cost: LITERAL_COST,
            });
        }

        candidates.sort_unstable_by_key(|candidate| candidate.cost);
        candidates
    }

    #[must_use]
    pub fn convert_best(&self, reading: &str) -> Option<Conversion> {
        if reading.is_empty() {
            return None;
        }

        let mut best_cost = vec![i32::MAX; reading.len() + 1];
        let mut previous: Vec<Option<Predecessor>> = vec![None; reading.len() + 1];
        best_cost[0] = 0;

        for start in reading
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(reading.len()))
        {
            let path_cost = best_cost[start];
            if path_cost == i32::MAX || start == reading.len() {
                continue;
            }

            let suffix = &reading[start..];
            for entry in self
                .entries
                .iter()
                .filter(|entry| suffix.starts_with(entry.reading))
            {
                let end = start + entry.reading.len();
                update_path(
                    &mut best_cost,
                    &mut previous,
                    start,
                    end,
                    path_cost.saturating_add(entry.word_cost),
                    entry.reading,
                    entry.surface,
                    entry.word_cost,
                );
            }

            let Some(character) = suffix.chars().next() else {
                continue;
            };
            let end = start + character.len_utf8();
            let literal = &reading[start..end];
            update_path(
                &mut best_cost,
                &mut previous,
                start,
                end,
                path_cost.saturating_add(UNKNOWN_COST),
                literal,
                literal,
                UNKNOWN_COST,
            );
        }

        let total_cost = best_cost[reading.len()];
        if total_cost == i32::MAX {
            return None;
        }

        let mut reversed = Vec::new();
        let mut cursor = reading.len();
        while cursor > 0 {
            let predecessor = previous[cursor].take()?;
            cursor = predecessor.start;
            reversed.push(Segment {
                reading: predecessor.reading,
                surface: predecessor.surface,
                cost: predecessor.segment_cost,
            });
        }
        reversed.reverse();

        let surface_capacity = reversed.iter().map(|segment| segment.surface.len()).sum();
        let mut surface = String::with_capacity(surface_capacity);
        for segment in &reversed {
            surface.push_str(&segment.surface);
        }

        Some(Conversion {
            surface,
            segments: reversed,
            cost: total_cost,
        })
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::bundled()
    }
}

#[derive(Clone, Debug)]
struct Predecessor {
    start: usize,
    reading: String,
    surface: String,
    segment_cost: i32,
}

#[allow(clippy::too_many_arguments)]
fn update_path(
    best_cost: &mut [i32],
    previous: &mut [Option<Predecessor>],
    start: usize,
    end: usize,
    total_cost: i32,
    reading: &str,
    surface: &str,
    segment_cost: i32,
) {
    if total_cost >= best_cost[end] {
        return;
    }

    best_cost[end] = total_cost;
    previous[end] = Some(Predecessor {
        start,
        reading: reading.to_owned(),
        surface: surface.to_owned(),
        segment_cost,
    });
}

const UNKNOWN_COST: i32 = 10_000;
const LITERAL_COST: i32 = 20_000;

const BUNDLED_ENTRIES: &[DictionaryEntry] = &[
    DictionaryEntry {
        reading: "きょう",
        surface: "今日",
        word_cost: 10,
    },
    DictionaryEntry {
        reading: "こんにちは",
        surface: "こんにちは",
        word_cost: 5,
    },
    DictionaryEntry {
        reading: "にほん",
        surface: "日本",
        word_cost: 10,
    },
    DictionaryEntry {
        reading: "にほん",
        surface: "二本",
        word_cost: 20,
    },
    DictionaryEntry {
        reading: "は",
        surface: "は",
        word_cost: 1,
    },
    DictionaryEntry {
        reading: "わたし",
        surface: "私",
        word_cost: 10,
    },
];

#[cfg(test)]
mod tests {
    use super::{Dictionary, DictionaryEntry};

    #[test]
    fn exact_candidates_are_ordered_by_cost() {
        let dictionary = Dictionary::bundled();
        let candidates = dictionary.candidates("にほん");

        assert_eq!(candidates[0].surface, "日本");
        assert_eq!(candidates[1].surface, "二本");
        assert_eq!(candidates[2].surface, "にほん");
    }

    #[test]
    fn viterbi_selects_best_segmented_path() {
        let dictionary = Dictionary::bundled();
        let conversion = dictionary.convert_best("わたしはにほん").unwrap();

        assert_eq!(conversion.surface, "私は日本");
        assert_eq!(conversion.cost, 21);
        assert_eq!(conversion.segments.len(), 3);
    }

    #[test]
    fn unknown_input_falls_back_without_data_loss() {
        let dictionary = Dictionary::bundled();
        let conversion = dictionary.convert_best("ねこ").unwrap();

        assert_eq!(conversion.surface, "ねこ");
        assert_eq!(conversion.segments.len(), 2);
    }

    #[test]
    fn lower_cost_path_wins_over_longer_entry() {
        let dictionary = Dictionary::new(vec![
            DictionaryEntry {
                reading: "あ",
                surface: "亜",
                word_cost: 10,
            },
            DictionaryEntry {
                reading: "い",
                surface: "伊",
                word_cost: 10,
            },
            DictionaryEntry {
                reading: "あい",
                surface: "愛",
                word_cost: 30,
            },
        ]);

        assert_eq!(dictionary.convert_best("あい").unwrap().surface, "亜伊");
    }

    #[test]
    fn empty_input_has_no_conversion() {
        assert!(Dictionary::bundled().convert_best("").is_none());
    }
}
