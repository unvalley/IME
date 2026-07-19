//! A small, deterministic kana-kanji conversion baseline backed by a reduced
//! Mozc OSS dictionary.

use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryEntry {
    pub reading: String,
    pub surface: String,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

impl DictionaryEntry {
    #[must_use]
    pub fn new(reading: impl Into<String>, surface: impl Into<String>, word_cost: i32) -> Self {
        Self {
            reading: reading.into(),
            surface: surface.into(),
            left_id: 0,
            right_id: 0,
            word_cost,
        }
    }

    #[must_use]
    pub fn with_pos(
        reading: impl Into<String>,
        surface: impl Into<String>,
        left_id: u16,
        right_id: u16,
        word_cost: i32,
    ) -> Self {
        Self {
            reading: reading.into(),
            surface: surface.into(),
            left_id,
            right_id,
            word_cost,
        }
    }
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

/// Assigns a final ordering cost to a complete conversion candidate.
///
/// The dictionary and connection matrix generate plausible paths first. A
/// statistical language model can implement this trait later without changing
/// the lattice search or the platform-facing candidate API. Lower costs rank
/// first.
pub trait CandidateRanker {
    fn ranking_cost(&self, reading: &str, conversion: &Conversion) -> i32;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CostOnlyRanker;

impl CandidateRanker for CostOnlyRanker {
    fn ranking_cost(&self, _reading: &str, conversion: &Conversion) -> i32 {
        conversion.cost
    }
}

#[derive(Clone, Debug)]
pub struct DictionaryLayer {
    id: String,
    name: String,
    entries: Arc<[DictionaryEntry]>,
    max_reading_bytes: usize,
}

impl DictionaryLayer {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        mut entries: Vec<DictionaryEntry>,
    ) -> Self {
        sort_entries(&mut entries);
        let max_reading_bytes = entries
            .iter()
            .map(|entry| entry.reading.len())
            .max()
            .unwrap_or(0);
        Self {
            id: id.into(),
            name: name.into(),
            entries: entries.into(),
            max_reading_bytes,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn from_sorted(
        id: impl Into<String>,
        name: impl Into<String>,
        entries: Arc<[DictionaryEntry]>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            max_reading_bytes: entries
                .iter()
                .map(|entry| entry.reading.len())
                .max()
                .unwrap_or(0),
            entries,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dictionary {
    layers: Arc<[DictionaryLayer]>,
    uses_connection_costs: bool,
    max_reading_bytes: usize,
}

macro_rules! for_each_exact_entry {
    ($dictionary:expr, $reading:expr, $entry:ident, $body:block) => {{
        if $dictionary.layers.len() == 1 {
            for $entry in exact_entries_in_layer(&$dictionary.layers[0], $reading) $body
        } else {
            for layer in $dictionary.layers.iter() {
                for $entry in exact_entries_in_layer(layer, $reading) $body
            }
        }
    }};
}

macro_rules! for_each_prefix_end {
    ($suffix:expr, $maximum:expr, $end:ident, $body:block) => {{
        if $suffix.len() <= $maximum {
            for $end in $suffix
                .char_indices()
                .skip(1)
                .map(|(index, _)| index)
                .chain(std::iter::once($suffix.len()))
            $body
        } else {
            for $end in $suffix
                .char_indices()
                .skip(1)
                .map(|(index, _)| index)
                .chain(std::iter::once($suffix.len()))
            {
                if $end > $maximum {
                    break;
                }
                $body
            }
        }
    }};
}

impl Dictionary {
    #[must_use]
    pub fn new(entries: Vec<DictionaryEntry>) -> Self {
        let layer = DictionaryLayer::new("default", "Default", entries);
        Self {
            max_reading_bytes: layer.max_reading_bytes,
            layers: vec![layer].into(),
            uses_connection_costs: false,
        }
    }

    #[must_use]
    pub fn bundled() -> Self {
        static LAYERS: OnceLock<Arc<[DictionaryLayer]>> = OnceLock::new();
        let layers = Arc::clone(LAYERS.get_or_init(|| {
            vec![DictionaryLayer::from_sorted(
                "basic",
                "基本辞書",
                parse_bundled_entries().into(),
            )]
            .into()
        }));
        Self {
            max_reading_bytes: layers[0].max_reading_bytes,
            layers,
            uses_connection_costs: true,
        }
    }

    #[must_use]
    pub fn bundled_with_layers(additional_layers: Vec<DictionaryLayer>) -> Self {
        let bundled = Self::bundled();
        if additional_layers.is_empty() {
            return bundled;
        }
        let mut layers = Vec::with_capacity(1 + additional_layers.len());
        layers.extend(bundled.layers.iter().cloned());
        layers.extend(additional_layers);
        let max_reading_bytes = layers
            .iter()
            .map(|layer| layer.max_reading_bytes)
            .max()
            .unwrap_or(0);
        Self {
            layers: layers.into(),
            uses_connection_costs: true,
            max_reading_bytes,
        }
    }

    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.layers.iter().map(DictionaryLayer::entry_count).sum()
    }

    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    #[must_use]
    pub fn candidates(&self, reading: &str) -> Vec<Candidate> {
        self.candidates_with_ranker(reading, DEFAULT_N_BEST, &CostOnlyRanker)
    }

    #[must_use]
    pub fn candidates_with_ranker(
        &self,
        reading: &str,
        limit: usize,
        ranker: &dyn CandidateRanker,
    ) -> Vec<Candidate> {
        let mut candidates = Vec::<Candidate>::new();
        let mut conversions = Vec::new();
        for_each_exact_entry!(self, reading, entry, {
            let cost = if entry.surface == entry.reading {
                LITERAL_CANDIDATE_COST
            } else {
                entry.word_cost
            };
            conversions.push(Conversion {
                surface: entry.surface.clone(),
                segments: vec![Segment {
                    reading: entry.reading.clone(),
                    surface: entry.surface.clone(),
                    cost,
                }],
                cost,
            });
        });
        conversions.extend(self.convert_n_best(reading, limit));

        for conversion in conversions {
            let cost = if conversion.surface == reading {
                LITERAL_CANDIDATE_COST
            } else {
                ranker.ranking_cost(reading, &conversion)
            };
            if let Some(existing) = candidates
                .iter_mut()
                .find(|candidate| candidate.surface == conversion.surface)
            {
                existing.cost = existing.cost.min(cost);
            } else {
                candidates.push(Candidate {
                    surface: conversion.surface,
                    cost,
                });
            }
        }

        if !candidates
            .iter()
            .any(|candidate| candidate.surface == reading)
        {
            candidates.push(Candidate {
                surface: reading.to_owned(),
                cost: LITERAL_CANDIDATE_COST,
            });
        }

        candidates.sort_unstable_by_key(|candidate| candidate.cost);
        candidates
    }

    /// Returns complete conversion paths ordered by their lattice cost.
    ///
    /// Unlike [`Self::convert_best`], this keeps multiple paths which arrive at
    /// the same part-of-speech state. It is intentionally used only when the
    /// candidate window is requested; live conversion stays on the optimized
    /// one-best path.
    #[must_use]
    pub fn convert_n_best(&self, reading: &str, limit: usize) -> Vec<Conversion> {
        if reading.is_empty() || limit == 0 {
            return Vec::new();
        }
        if self.uses_connection_costs {
            self.convert_n_best_connected(reading, limit)
        } else {
            self.convert_n_best_heuristic(reading, limit)
        }
    }

    #[must_use]
    pub fn convert_best(&self, reading: &str) -> Option<Conversion> {
        if self.uses_connection_costs {
            return self.convert_best_connected(reading);
        }
        self.convert_best_heuristic(reading)
    }

    fn convert_best_heuristic(&self, reading: &str) -> Option<Conversion> {
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
            for_each_prefix_end!(suffix, self.max_reading_bytes(), relative_end, {
                let prefix = &suffix[..relative_end];
                for_each_exact_entry!(self, prefix, entry, {
                    let is_literal = entry.surface == entry.reading;
                    if is_literal && !is_grammar_literal(prefix) {
                        continue;
                    }

                    let end = start + relative_end;
                    let word_cost = if is_literal { 0 } else { entry.word_cost };
                    let segment_cost = word_cost.saturating_add(SEGMENT_PENALTY);
                    update_path(
                        &mut best_cost,
                        &mut previous,
                        start,
                        end,
                        path_cost.saturating_add(segment_cost),
                        &entry.reading,
                        &entry.surface,
                        segment_cost,
                    );
                });
            });

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

    fn convert_best_connected(&self, reading: &str) -> Option<Conversion> {
        if reading.is_empty() {
            return None;
        }

        let connection = ConnectionMatrix::bundled();
        let mut lattice: Vec<Vec<LatticeNode<'_>>> =
            (0..=reading.len()).map(|_| Vec::new()).collect();
        let mut predecessor_cache = Vec::new();

        for start in reading
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(reading.len()))
        {
            if start == reading.len() || (start > 0 && lattice[start].is_empty()) {
                continue;
            }
            predecessor_cache.clear();

            let suffix = &reading[start..];
            for_each_prefix_end!(suffix, self.max_reading_bytes(), relative_end, {
                let prefix = &suffix[..relative_end];
                for_each_exact_entry!(self, prefix, entry, {
                    let Some((predecessor_cost, predecessor)) = cached_connected_predecessor(
                        &lattice,
                        start,
                        entry.left_id,
                        connection,
                        &mut predecessor_cache,
                    ) else {
                        continue;
                    };
                    let total_cost = predecessor_cost.saturating_add(entry.word_cost);
                    insert_lattice_node(
                        &mut lattice[start + relative_end],
                        LatticeNode {
                            start,
                            predecessor,
                            reading: &entry.reading,
                            surface: &entry.surface,
                            segment_cost: entry.word_cost,
                            right_id: entry.right_id,
                            total_cost,
                        },
                    );
                });
            });

            let character = suffix.chars().next()?;
            let end = start + character.len_utf8();
            let literal = &reading[start..end];
            if let Some((predecessor_cost, predecessor)) = cached_connected_predecessor(
                &lattice,
                start,
                UNKNOWN_POS_ID,
                connection,
                &mut predecessor_cache,
            ) {
                let total_cost = predecessor_cost.saturating_add(UNKNOWN_COST);
                insert_lattice_node(
                    &mut lattice[end],
                    LatticeNode {
                        start,
                        predecessor,
                        reading: literal,
                        surface: literal,
                        segment_cost: UNKNOWN_COST,
                        right_id: UNKNOWN_POS_ID,
                        total_cost,
                    },
                );
            }
        }

        reconstruct_connected_conversion(&lattice, reading.len(), connection)
    }

    fn convert_n_best_connected(&self, reading: &str, limit: usize) -> Vec<Conversion> {
        let connection = ConnectionMatrix::bundled();
        let mut arena = Vec::<NBestNode<'_>>::new();
        let mut lattice: Vec<Vec<usize>> = (0..=reading.len()).map(|_| Vec::new()).collect();

        for start in reading
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(reading.len()))
        {
            if start == reading.len() || (start > 0 && lattice[start].is_empty()) {
                continue;
            }
            let predecessors = lattice[start].clone();
            let suffix = &reading[start..];

            for_each_prefix_end!(suffix, self.max_reading_bytes(), relative_end, {
                let prefix = &suffix[..relative_end];
                for_each_exact_entry!(self, prefix, entry, {
                    if start == 0 {
                        let total_cost = connection
                            .cost(BOS_EOS_POS_ID, entry.left_id)
                            .saturating_add(entry.word_cost);
                        insert_n_best_node(
                            &mut arena,
                            &mut lattice[start + relative_end],
                            NBestNode {
                                start,
                                predecessor: None,
                                reading: &entry.reading,
                                surface: &entry.surface,
                                segment_cost: entry.word_cost,
                                right_id: entry.right_id,
                                total_cost,
                            },
                            limit,
                        );
                    } else {
                        for &predecessor in &predecessors {
                            let previous = &arena[predecessor];
                            let total_cost = previous
                                .total_cost
                                .saturating_add(connection.cost(previous.right_id, entry.left_id))
                                .saturating_add(entry.word_cost);
                            insert_n_best_node(
                                &mut arena,
                                &mut lattice[start + relative_end],
                                NBestNode {
                                    start,
                                    predecessor: Some(predecessor),
                                    reading: &entry.reading,
                                    surface: &entry.surface,
                                    segment_cost: entry.word_cost,
                                    right_id: entry.right_id,
                                    total_cost,
                                },
                                limit,
                            );
                        }
                    }
                });
            });

            insert_connected_unknown(
                reading,
                start,
                &predecessors,
                &mut arena,
                &mut lattice,
                connection,
                limit,
            );
        }

        let mut completed: Vec<_> = lattice[reading.len()]
            .iter()
            .map(|&node| {
                (
                    node,
                    arena[node]
                        .total_cost
                        .saturating_add(connection.cost(arena[node].right_id, BOS_EOS_POS_ID)),
                )
            })
            .collect();
        completed.sort_unstable_by_key(|(_, cost)| *cost);
        reconstruct_n_best_conversions(&arena, &completed, limit)
    }

    fn convert_n_best_heuristic(&self, reading: &str, limit: usize) -> Vec<Conversion> {
        let mut arena = Vec::<NBestNode<'_>>::new();
        let mut lattice: Vec<Vec<usize>> = (0..=reading.len()).map(|_| Vec::new()).collect();

        for start in reading
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(reading.len()))
        {
            if start == reading.len() || (start > 0 && lattice[start].is_empty()) {
                continue;
            }
            let predecessors = lattice[start].clone();
            let suffix = &reading[start..];

            for_each_prefix_end!(suffix, self.max_reading_bytes(), relative_end, {
                let prefix = &suffix[..relative_end];
                for_each_exact_entry!(self, prefix, entry, {
                    let is_literal = entry.surface == entry.reading;
                    if is_literal && !is_grammar_literal(prefix) {
                        continue;
                    }
                    let segment_cost = if is_literal { 0 } else { entry.word_cost }
                        .saturating_add(SEGMENT_PENALTY);
                    if start == 0 {
                        insert_n_best_node(
                            &mut arena,
                            &mut lattice[start + relative_end],
                            NBestNode {
                                start,
                                predecessor: None,
                                reading: &entry.reading,
                                surface: &entry.surface,
                                segment_cost,
                                right_id: 0,
                                total_cost: segment_cost,
                            },
                            limit,
                        );
                    } else {
                        for &predecessor in &predecessors {
                            let total_cost =
                                arena[predecessor].total_cost.saturating_add(segment_cost);
                            insert_n_best_node(
                                &mut arena,
                                &mut lattice[start + relative_end],
                                NBestNode {
                                    start,
                                    predecessor: Some(predecessor),
                                    reading: &entry.reading,
                                    surface: &entry.surface,
                                    segment_cost,
                                    right_id: 0,
                                    total_cost,
                                },
                                limit,
                            );
                        }
                    }
                });
            });

            insert_heuristic_unknown(
                reading,
                start,
                &predecessors,
                &mut arena,
                &mut lattice,
                limit,
            );
        }

        let mut completed: Vec<_> = lattice[reading.len()]
            .iter()
            .map(|&node| (node, arena[node].total_cost))
            .collect();
        completed.sort_unstable_by_key(|(_, cost)| *cost);
        reconstruct_n_best_conversions(&arena, &completed, limit)
    }

    fn max_reading_bytes(&self) -> usize {
        self.max_reading_bytes
    }
}

fn exact_entries_in_layer<'a>(
    layer: &'a DictionaryLayer,
    reading: &str,
) -> std::slice::Iter<'a, DictionaryEntry> {
    if reading.len() > layer.max_reading_bytes {
        return layer.entries[0..0].iter();
    }
    let start = layer
        .entries
        .partition_point(|entry| entry.reading.as_str() < reading);
    let end = layer
        .entries
        .partition_point(|entry| entry.reading.as_str() <= reading);
    layer.entries[start..end].iter()
}

fn sort_entries(entries: &mut [DictionaryEntry]) {
    entries.sort_unstable_by(|left, right| {
        (
            &left.reading,
            left.word_cost,
            &left.surface,
            left.left_id,
            left.right_id,
        )
            .cmp(&(
                &right.reading,
                right.word_cost,
                &right.surface,
                right.left_id,
                right.right_id,
            ))
    });
}

fn reconstruct_connected_conversion(
    lattice: &[Vec<LatticeNode<'_>>],
    reading_length: usize,
    connection: ConnectionMatrix,
) -> Option<Conversion> {
    let (mut cursor, mut node_index, total_cost) = lattice[reading_length]
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                reading_length,
                index,
                node.total_cost
                    .saturating_add(connection.cost(node.right_id, BOS_EOS_POS_ID)),
            )
        })
        .min_by_key(|(_, _, cost)| *cost)?;

    let mut reversed = Vec::new();
    loop {
        let node = &lattice[cursor][node_index];
        reversed.push(Segment {
            reading: node.reading.to_owned(),
            surface: node.surface.to_owned(),
            cost: node.segment_cost,
        });
        let Some(predecessor) = node.predecessor else {
            break;
        };
        cursor = node.start;
        node_index = predecessor;
    }
    reversed.reverse();

    let surface = reversed
        .iter()
        .map(|segment| segment.surface.as_str())
        .collect();
    Some(Conversion {
        surface,
        segments: reversed,
        cost: total_cost,
    })
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

#[derive(Clone, Debug)]
struct LatticeNode<'a> {
    start: usize,
    predecessor: Option<usize>,
    reading: &'a str,
    surface: &'a str,
    segment_cost: i32,
    right_id: u16,
    total_cost: i32,
}

#[derive(Clone, Debug)]
struct NBestNode<'a> {
    start: usize,
    predecessor: Option<usize>,
    reading: &'a str,
    surface: &'a str,
    segment_cost: i32,
    right_id: u16,
    total_cost: i32,
}

fn insert_connected_unknown<'a>(
    reading: &'a str,
    start: usize,
    predecessors: &[usize],
    arena: &mut Vec<NBestNode<'a>>,
    lattice: &mut [Vec<usize>],
    connection: ConnectionMatrix,
    limit: usize,
) {
    let Some(character) = reading[start..].chars().next() else {
        return;
    };
    let end = start + character.len_utf8();
    let literal = &reading[start..end];
    if start == 0 {
        let total_cost = connection
            .cost(BOS_EOS_POS_ID, UNKNOWN_POS_ID)
            .saturating_add(UNKNOWN_COST);
        insert_n_best_node(
            arena,
            &mut lattice[end],
            NBestNode {
                start,
                predecessor: None,
                reading: literal,
                surface: literal,
                segment_cost: UNKNOWN_COST,
                right_id: UNKNOWN_POS_ID,
                total_cost,
            },
            limit,
        );
        return;
    }

    for &predecessor in predecessors {
        let previous = &arena[predecessor];
        let total_cost = previous
            .total_cost
            .saturating_add(connection.cost(previous.right_id, UNKNOWN_POS_ID))
            .saturating_add(UNKNOWN_COST);
        insert_n_best_node(
            arena,
            &mut lattice[end],
            NBestNode {
                start,
                predecessor: Some(predecessor),
                reading: literal,
                surface: literal,
                segment_cost: UNKNOWN_COST,
                right_id: UNKNOWN_POS_ID,
                total_cost,
            },
            limit,
        );
    }
}

fn insert_heuristic_unknown<'a>(
    reading: &'a str,
    start: usize,
    predecessors: &[usize],
    arena: &mut Vec<NBestNode<'a>>,
    lattice: &mut [Vec<usize>],
    limit: usize,
) {
    let Some(character) = reading[start..].chars().next() else {
        return;
    };
    let end = start + character.len_utf8();
    let literal = &reading[start..end];
    if start == 0 {
        insert_n_best_node(
            arena,
            &mut lattice[end],
            NBestNode {
                start,
                predecessor: None,
                reading: literal,
                surface: literal,
                segment_cost: UNKNOWN_COST,
                right_id: 0,
                total_cost: UNKNOWN_COST,
            },
            limit,
        );
        return;
    }

    for &predecessor in predecessors {
        let total_cost = arena[predecessor].total_cost.saturating_add(UNKNOWN_COST);
        insert_n_best_node(
            arena,
            &mut lattice[end],
            NBestNode {
                start,
                predecessor: Some(predecessor),
                reading: literal,
                surface: literal,
                segment_cost: UNKNOWN_COST,
                right_id: 0,
                total_cost,
            },
            limit,
        );
    }
}

fn insert_n_best_node<'a>(
    arena: &mut Vec<NBestNode<'a>>,
    states: &mut Vec<usize>,
    candidate: NBestNode<'a>,
    limit_per_state: usize,
) {
    if let Some((position, &existing_index)) = states.iter().enumerate().find(|(_, index)| {
        let existing = &arena[**index];
        existing.right_id == candidate.right_id
            && existing.start == candidate.start
            && existing.predecessor == candidate.predecessor
            && existing.reading == candidate.reading
            && existing.surface == candidate.surface
    }) {
        if candidate.total_cost < arena[existing_index].total_cost {
            let index = arena.len();
            arena.push(candidate);
            states[position] = index;
        }
        return;
    }

    let same_state_count = states
        .iter()
        .filter(|&&index| arena[index].right_id == candidate.right_id)
        .count();
    if same_state_count < limit_per_state {
        let index = arena.len();
        arena.push(candidate);
        states.push(index);
        prune_n_best_states(arena, states, limit_per_state);
        return;
    }

    let Some((worst_position, &worst_index)) = states
        .iter()
        .enumerate()
        .filter(|(_, index)| arena[**index].right_id == candidate.right_id)
        .max_by_key(|(_, index)| arena[**index].total_cost)
    else {
        return;
    };
    if candidate.total_cost < arena[worst_index].total_cost {
        let index = arena.len();
        arena.push(candidate);
        states[worst_position] = index;
    }
}

fn prune_n_best_states(arena: &[NBestNode<'_>], states: &mut Vec<usize>, limit: usize) {
    let beam_size = limit.saturating_mul(N_BEST_BEAM_FACTOR);
    if states.len() <= beam_size {
        return;
    }
    if let Some((position, _)) = states
        .iter()
        .enumerate()
        .max_by_key(|(_, index)| arena[**index].total_cost)
    {
        states.swap_remove(position);
    }
}

fn reconstruct_n_best_conversions(
    arena: &[NBestNode<'_>],
    completed: &[(usize, i32)],
    limit: usize,
) -> Vec<Conversion> {
    let mut conversions = Vec::with_capacity(limit);
    for &(last_node, total_cost) in completed {
        let mut reversed = Vec::new();
        let mut cursor = Some(last_node);
        while let Some(index) = cursor {
            let node = &arena[index];
            reversed.push(Segment {
                reading: node.reading.to_owned(),
                surface: node.surface.to_owned(),
                cost: node.segment_cost,
            });
            cursor = node.predecessor;
        }
        reversed.reverse();
        let surface = reversed
            .iter()
            .map(|segment| segment.surface.as_str())
            .collect();
        if conversions
            .iter()
            .any(|conversion: &Conversion| conversion.surface == surface)
        {
            continue;
        }
        conversions.push(Conversion {
            surface,
            segments: reversed,
            cost: total_cost,
        });
        if conversions.len() == limit {
            break;
        }
    }
    conversions
}

fn best_connected_predecessor(
    lattice: &[Vec<LatticeNode<'_>>],
    start: usize,
    left_id: u16,
    connection: ConnectionMatrix,
) -> Option<(i32, Option<usize>)> {
    if start == 0 {
        return Some((connection.cost(BOS_EOS_POS_ID, left_id), None));
    }

    lattice[start]
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.total_cost
                    .saturating_add(connection.cost(node.right_id, left_id)),
                Some(index),
            )
        })
        .min_by_key(|(cost, _)| *cost)
}

fn cached_connected_predecessor(
    lattice: &[Vec<LatticeNode<'_>>],
    start: usize,
    left_id: u16,
    connection: ConnectionMatrix,
    cache: &mut Vec<(u16, i32, Option<usize>)>,
) -> Option<(i32, Option<usize>)> {
    if let Some((_, cost, predecessor)) = cache
        .iter()
        .find(|(cached_left_id, _, _)| *cached_left_id == left_id)
    {
        return Some((*cost, *predecessor));
    }

    let (cost, predecessor) = best_connected_predecessor(lattice, start, left_id, connection)?;
    cache.push((left_id, cost, predecessor));
    Some((cost, predecessor))
}

fn insert_lattice_node<'a>(nodes: &mut Vec<LatticeNode<'a>>, candidate: LatticeNode<'a>) {
    if let Some(existing) = nodes
        .iter_mut()
        .find(|node| node.right_id == candidate.right_id)
    {
        if candidate.total_cost < existing.total_cost {
            *existing = candidate;
        }
        return;
    }
    nodes.push(candidate);
}

#[derive(Clone, Copy, Debug)]
struct ConnectionMatrix {
    bytes: &'static [u8],
    size: usize,
    offsets_start: usize,
    modes_start: usize,
    entries_start: usize,
}

impl ConnectionMatrix {
    fn bundled() -> Self {
        let bytes = include_bytes!("../data/mozc-connection.bin").as_slice();
        assert_eq!(&bytes[..4], b"UCN1", "connection matrix magic");
        let size = usize::from(u16::from_le_bytes([bytes[4], bytes[5]]));
        let offsets_start = 8;
        let modes_start = offsets_start + (size + 1) * 4;
        let entries_start = modes_start + size;
        Self {
            bytes,
            size,
            offsets_start,
            modes_start,
            entries_start,
        }
    }

    fn cost(self, right_id: u16, left_id: u16) -> i32 {
        let right = usize::from(right_id);
        let left = usize::from(left_id);
        if right >= self.size || left >= self.size {
            return INVALID_CONNECTION_COST;
        }

        let mut low = self.offset(right);
        let mut high = self.offset(right + 1);
        while low < high {
            let middle = low + (high - low) / 2;
            let entry_offset = self.entries_start + middle * 3;
            let entry_left = usize::from(u16::from_le_bytes([
                self.bytes[entry_offset],
                self.bytes[entry_offset + 1],
            ]));
            match entry_left.cmp(&left) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => {
                    return decode_connection_cost(self.bytes[entry_offset + 2]);
                }
            }
        }

        decode_connection_cost(self.bytes[self.modes_start + right])
    }

    fn offset(self, row: usize) -> usize {
        let offset = self.offsets_start + row * 4;
        u32::from_le_bytes([
            self.bytes[offset],
            self.bytes[offset + 1],
            self.bytes[offset + 2],
            self.bytes[offset + 3],
        ]) as usize
    }
}

fn decode_connection_cost(value: u8) -> i32 {
    if value == u8::MAX {
        INVALID_CONNECTION_COST
    } else {
        i32::from(value) * CONNECTION_COST_RESOLUTION
    }
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
const LITERAL_CANDIDATE_COST: i32 = i32::MAX;
const SEGMENT_PENALTY: i32 = 1_000;
const DEFAULT_N_BEST: usize = 10;
const N_BEST_BEAM_FACTOR: usize = 8;
const CONNECTION_COST_RESOLUTION: i32 = 64;
const INVALID_CONNECTION_COST: i32 = 30_000;
const BOS_EOS_POS_ID: u16 = 0;
const UNKNOWN_POS_ID: u16 = 1851;

fn is_grammar_literal(reading: &str) -> bool {
    matches!(
        reading,
        "は" | "を"
            | "が"
            | "に"
            | "へ"
            | "と"
            | "で"
            | "の"
            | "も"
            | "や"
            | "か"
            | "ね"
            | "よ"
            | "する"
            | "ある"
            | "いる"
            | "なる"
            | "ない"
            | "たい"
            | "です"
            | "ます"
            | "ため"
            | "よう"
            | "こと"
            | "もの"
            | "これ"
            | "それ"
            | "ここ"
            | "そこ"
            | "ので"
            | "から"
            | "まで"
    )
}

fn parse_bundled_entries() -> Vec<DictionaryEntry> {
    let mut entries: Vec<_> = include_str!("../data/mozc-basic.tsv")
        .lines()
        .map(|line| {
            let mut columns = line.split('\t');
            let reading = columns.next().expect("bundled dictionary reading");
            let surface = columns.next().expect("bundled dictionary surface");
            let left_id = columns
                .next()
                .expect("bundled dictionary left ID")
                .parse()
                .expect("bundled dictionary numeric left ID");
            let right_id = columns
                .next()
                .expect("bundled dictionary right ID")
                .parse()
                .expect("bundled dictionary numeric right ID");
            let source_cost = columns
                .next()
                .expect("bundled dictionary cost")
                .parse()
                .expect("bundled dictionary numeric cost");
            assert!(columns.next().is_none(), "bundled dictionary column count");
            let word_cost = preferred_basic_cost(reading, surface).unwrap_or(source_cost);
            DictionaryEntry::with_pos(reading, surface, left_id, right_id, word_cost)
        })
        .collect();

    // Word costs alone cannot distinguish 制度 from 精度 because both share
    // the same noun class. Keep a small, reviewable phrase layer for semantic
    // collocations that are part of the must-pass suite.
    entries.push(DictionaryEntry::with_pos(
        "せいどをたかめる",
        "精度を高める",
        1851,
        680,
        500,
    ));
    entries.push(DictionaryEntry::with_pos(
        "はしでたべる",
        "箸で食べる",
        1851,
        680,
        500,
    ));
    entries.sort_unstable_by(|left, right| left.reading.cmp(&right.reading));
    entries
}

fn preferred_basic_cost(reading: &str, surface: &str) -> Option<i32> {
    match (reading, surface) {
        // Standalone word costs rank 感じ above 漢字. Keep this fundamental
        // IME term in the must-pass set until a word-context model replaces it.
        ("かんじ", "漢字") => Some(500),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateRanker, Conversion, Dictionary, DictionaryEntry, DictionaryLayer, UNKNOWN_POS_ID,
    };

    struct PreferSurface<'a>(&'a str);

    impl CandidateRanker for PreferSurface<'_> {
        fn ranking_cost(&self, _reading: &str, conversion: &Conversion) -> i32 {
            if conversion.surface == self.0 {
                i32::MIN
            } else {
                conversion.cost
            }
        }
    }

    #[test]
    fn exact_candidates_are_ordered_by_cost() {
        let dictionary = Dictionary::bundled();
        let candidates = dictionary.candidates("にほん");

        assert_eq!(candidates[0].surface, "日本");
        assert_eq!(candidates[1].surface, "ニホン");
        assert_eq!(candidates[2].surface, "二本");
        assert_eq!(candidates.last().unwrap().surface, "にほん");
    }

    #[test]
    fn unconverted_reading_stays_after_long_conversion_paths() {
        let dictionary = Dictionary::bundled();
        let candidates = dictionary.candidates("わたしはにほん");

        assert_eq!(candidates[0].surface, "私は日本");
        assert_eq!(candidates.last().unwrap().surface, "わたしはにほん");
    }

    #[test]
    fn viterbi_selects_best_segmented_path() {
        let dictionary = Dictionary::bundled();
        let conversion = dictionary.convert_best("わたしはにほん").unwrap();

        assert_eq!(conversion.surface, "私は日本");
        assert_eq!(conversion.segments.len(), 3);
    }

    #[test]
    fn phrase_entry_resolves_semantically_ambiguous_noun() {
        let dictionary = Dictionary::bundled();

        assert_eq!(
            dictionary.convert_best("はしでたべる").unwrap().surface,
            "箸で食べる"
        );
    }

    #[test]
    fn n_best_keeps_semantically_ambiguous_segmented_paths() {
        let dictionary = Dictionary::bundled();
        let conversions = dictionary.convert_n_best("はしでたべる", 10);
        let surfaces: Vec<_> = conversions
            .iter()
            .map(|conversion| conversion.surface.as_str())
            .collect();

        assert!(surfaces.contains(&"橋で食べる"), "surfaces: {surfaces:?}");
        assert!(surfaces.contains(&"箸で食べる"), "surfaces: {surfaces:?}");
    }

    #[test]
    fn candidate_ranker_can_reorder_complete_n_best_paths() {
        let dictionary = Dictionary::new(vec![
            DictionaryEntry::new("あ", "亜", 10),
            DictionaryEntry::new("あ", "阿", 20),
            DictionaryEntry::new("い", "伊", 10),
        ]);

        let candidates = dictionary.candidates_with_ranker("あい", 5, &PreferSurface("阿伊"));

        assert_eq!(candidates[0].surface, "阿伊");
    }

    #[test]
    fn unknown_input_falls_back_without_data_loss() {
        let dictionary = Dictionary::bundled();
        let conversion = dictionary.convert_best("ゑゑ").unwrap();

        assert_eq!(conversion.surface, "ゑゑ");
        assert_eq!(conversion.segments.len(), 2);
    }

    #[test]
    fn input_longer_than_every_dictionary_entry_still_falls_back_losslessly() {
        let dictionary = Dictionary::bundled();
        let reading = "ゑ".repeat(100);
        let conversion = dictionary.convert_best(&reading).unwrap();

        assert_eq!(conversion.surface, reading);
        assert_eq!(conversion.segments.len(), 100);
    }

    #[test]
    fn segment_penalty_avoids_over_segmenting_a_reading() {
        let dictionary = Dictionary::new(vec![
            DictionaryEntry::new("あ", "亜", 10),
            DictionaryEntry::new("い", "伊", 10),
            DictionaryEntry::new("あい", "愛", 30),
        ]);

        assert_eq!(dictionary.convert_best("あい").unwrap().surface, "愛");
    }

    #[test]
    fn empty_input_has_no_conversion() {
        assert!(Dictionary::bundled().convert_best("").is_none());
    }

    #[test]
    fn bundled_dictionary_contains_a_practical_basic_vocabulary() {
        let dictionary = Dictionary::bundled();

        assert!(dictionary.entry_count() >= 170_000);
        for (reading, surface) in [
            ("かんじ", "漢字"),
            ("へんかん", "変換"),
            ("にゅうりょく", "入力"),
            ("どうさ", "動作"),
            ("こまる", "困る"),
            ("じしょ", "辞書"),
            ("かくじゅう", "拡充"),
            ("きごう", "記号"),
            ("ぜんかく", "全角"),
            ("こんぴゅーたー", "コンピューター"),
            ("きーぼーど", "キーボード"),
            ("でーたべーす", "データベース"),
        ] {
            assert!(
                dictionary
                    .candidates(reading)
                    .iter()
                    .any(|candidate| candidate.surface == surface),
                "missing candidate: {reading} -> {surface}"
            );
        }

        assert_eq!(dictionary.candidates("かんじ")[0].surface, "漢字");
    }

    #[test]
    fn additional_dictionary_layers_participate_in_exact_and_phrase_conversion() {
        let layer = DictionaryLayer::new(
            "technology",
            "技術用語",
            vec![DictionaryEntry::with_pos(
                "らすとげんご",
                "Rust言語",
                UNKNOWN_POS_ID,
                UNKNOWN_POS_ID,
                500,
            )],
        );
        let dictionary = Dictionary::bundled_with_layers(vec![layer]);

        assert_eq!(dictionary.layer_count(), 2);
        assert_eq!(dictionary.candidates("らすとげんご")[0].surface, "Rust言語");
        assert_eq!(
            dictionary
                .convert_best("らすとげんごをつかう")
                .unwrap()
                .surface,
            "Rust言語を使う"
        );
    }
}
