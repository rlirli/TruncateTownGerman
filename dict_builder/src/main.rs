use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs::{self, File},
    io::{self, BufRead},
    ops::AddAssign,
    path::PathBuf,
};

use dashmap::DashMap;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

const WORD_DEFINITIONS_PATH: &str = "../word_definitions/valid_german_words.txt";
const OBJECTIONABLE_PATH: &str = "../word_definitions/objectionable.json";

type WordFrequency = usize;

struct FrequencyConfig {
    path: &'static str,
    min_word_frequency: usize,
    separator: char,
    skip_first_line: bool,
}

const WORD_FREQUENCY_CORPORA: [FrequencyConfig; 3] = [
    FrequencyConfig {
        path: "support_data/de_full.txt",
        min_word_frequency: 5,
        separator: ' ',
        skip_first_line: false,
    },
    FrequencyConfig {
        path: "support_data/decow_wordfreq_cistem.csv",
        min_word_frequency: 50,
        separator: ',',
        skip_first_line: true,
    },
    FrequencyConfig {
        path: "support_data/de_wiki_wordrank.txt",
        min_word_frequency: 25,
        separator: '\t',
        skip_first_line: false,
    },
];

/// Normalize a German word to ASCII lowercase by replacing umlauts with digraphs.
fn normalize_german_umlauts(word: &str) -> String {
    word.to_lowercase()
        .replace("ä", "ae")
        .replace("ö", "oe")
        .replace("ü", "ue")
        .replace("ß", "ss")
}

/// Primary determiner for which words do and do not qualify for inclusion in Truncate's validity dictionary.
fn should_include_word(
    word: &String,
    word_frequency: WordFrequency,
    min_word_frequency: usize,
) -> bool {
    // One-letter words in Truncate can be a surprise, exclude them.
    if word.len() < 2 {
        return false;
    }
    // Truncate is ASCII-only — this also helps cut out proper names and words with punctuation
    if !word.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    // Filter out words that are too rare in the frequency corpus
    if word_frequency < min_word_frequency {
        return false;
    }
    true
}

/// Load word frequencies from configured files, and return merged frequencies and candidate list.
fn load_frequencies_and_candidates(
    valid_words: &BTreeSet<String>,
) -> (BTreeMap<String, f32>, BTreeSet<String>) {
    let mut frequency_lookup: BTreeMap<String, f32> = BTreeMap::new();
    let mut candidate_word_list: BTreeSet<String> = BTreeSet::new();

    for config in WORD_FREQUENCY_CORPORA.iter() {
        println!("Loading word frequencies from {}", config.path);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(config.path);
        let Ok(file) = File::open(&path) else {
            println!(
                "Warning: Could not open {}. Did you run setup_data.sh?",
                config.path
            );
            continue;
        };

        // Stream the file to keep memory usage low.
        let mut raw_counts = Vec::new();
        let mut valid_matched_words = Vec::new();

        let mut lines = io::BufReader::new(file).lines().flatten();

        if config.skip_first_line {
            lines.next(); // Skip header
        }

        for line in lines {
            let Some((word, count_str)) = line.split_once(config.separator) else {
                continue;
            };
            let Ok(raw_count) = count_str.trim().parse::<usize>() else {
                continue;
            };

            raw_counts.push(raw_count);

            let cleaned = normalize_german_umlauts(word);
            if should_include_word(&cleaned, raw_count, config.min_word_frequency) {
                // To keep peak memory memory optimization, only keep strings that are in our valid words dictionary
                if valid_words.contains(&cleaned) {
                    valid_matched_words.push((cleaned, raw_count));
                }
            }
        }

        println!("Recalculating frequency ranks for {}", config.path);
        raw_counts.sort_unstable(); // Sort ASC for binary search
        let total_words = raw_counts.len() as f32;

        for (cleaned, raw_count) in valid_matched_words {
            // Find how many words have a strictly smaller count
            let rank = raw_counts.partition_point(|&c| c < raw_count);
            let freq = rank as f32 / total_words;

            // Merge keeping the maximum relative frequency
            frequency_lookup
                .entry(cleaned.clone())
                .and_modify(|e| *e = f32::max(*e, freq))
                .or_insert(freq);

            candidate_word_list.insert(cleaned);
        }
    }

    (frequency_lookup, candidate_word_list)
}

fn load_valid_german_words() -> BTreeSet<String> {
    println!("Loading valid German words from file");
    let file = File::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORD_DEFINITIONS_PATH),
    )
    .expect(&format!("{WORD_DEFINITIONS_PATH} should exist. Make sure to run `npm start` in word_definitions/ first!"));

    BTreeSet::from_iter(
        io::BufReader::new(file)
            .lines()
            .flatten()
            .map(|line| normalize_german_umlauts(&line)),
    )
}

fn load_additions() -> BTreeSet<String> {
    println!("Loading additional data from files");

    let files = ["support_data/tranche_german_1_add.txt"].map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("add files should exist")
    });

    BTreeSet::from_iter(
        files
            .iter()
            .flat_map(|f| io::BufReader::new(f).lines().flatten())
            .map(|line| normalize_german_umlauts(&line)),
    )
}

fn load_removals() -> BTreeSet<String> {
    println!("Loading removal data from files");

    let files = ["support_data/tranche_german_1_del.txt"].map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("del files should exist")
    });

    BTreeSet::from_iter(
        files
            .iter()
            .flat_map(|f| io::BufReader::new(f).lines().flatten())
            .map(|line| normalize_german_umlauts(&line)),
    )
}

fn load_objectionable() -> Vec<String> {
    let input = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(OBJECTIONABLE_PATH),
    )
    .expect(&format!("{OBJECTIONABLE_PATH} should exist. Make sure to run `npm start` in word_definitions/ first!"));
    serde_json::from_slice(&input[..]).expect("objectionable.json should be the expected JSON")
}

fn score_extension_value(target_len: usize, larger_len: usize) -> usize {
    let diff = larger_len - target_len;
    if diff >= 5 {
        1
    } else {
        (5 - diff).pow(2)
    }
}

fn main() {
    println!("Starting the dict builder");

    // To help filter out less desired words, we require words to _also_ be in the list of German word definitions.
    let valid_words = load_valid_german_words();

    let (frequency_lookup, candidate_word_list) = load_frequencies_and_candidates(&valid_words);

    let mut final_wordlist: BTreeSet<_> = valid_words.intersection(&candidate_word_list).collect();

    println!(
        "Filtered out {} words lacking a real definition.",
        candidate_word_list.len() - final_wordlist.len()
    );

    let additions = load_additions();
    final_wordlist.extend(additions.iter());

    let removals = load_removals();
    for removal in removals {
        final_wordlist.remove(&removal);
    }

    println!("{} words in the total set.", final_wordlist.len());
    println!("Calculating word substring counts");

    struct WordData {
        substring_score: usize,
        frequency: f32,
        objectionable: bool,
    }

    // Convert BTreeSet to sorted Vec for efficient parallel iteration (cache-friendly layout)
    let final_wordlist_vec: Vec<String> = final_wordlist.into_iter().cloned().collect();

    // Pre-build a reversed-word index for suffix lookups: (reversed_word, original_index)
    // Sorted by reversed_word so we can binary-search for suffix matches.
    let mut reversed_index: Vec<(String, usize)> = final_wordlist_vec
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let rev: String = w.chars().rev().collect();
            (rev, i)
        })
        .collect();
    reversed_index.sort_unstable();

    let backprop_points: DashMap<usize, usize> = DashMap::new();
    let objectionable: HashSet<String> = load_objectionable().into_iter().collect();

    let mut scored_word_list: Vec<_> = final_wordlist_vec
        .par_iter()
        .enumerate()
        .map(|(word_idx, word)| {
            let frequency = frequency_lookup.get(word).cloned().unwrap_or(0.0);
            let word_len = word.len();

            let mut links: Vec<(usize, usize)> = Vec::new();

            // --- Prefix matches: words that start with `word` ---
            // Since final_wordlist_vec is sorted, all words starting with `word` are contiguous.
            let prefix_start = final_wordlist_vec.partition_point(|w| w.as_str() < word.as_str());
            // Compute exclusive upper bound for the prefix range
            let mut upper_bytes = word.as_bytes().to_vec();
            if let Some(last) = upper_bytes.last_mut() {
                // Safe because all chars are ascii lowercase (max 'z' = 122, +1 = 123 = '{')
                *last += 1;
            }
            let upper_bound = unsafe { String::from_utf8_unchecked(upper_bytes) };
            let prefix_end =
                final_wordlist_vec.partition_point(|w| w.as_str() < upper_bound.as_str());

            for (idx, candidate) in final_wordlist_vec[prefix_start..prefix_end]
                .iter()
                .enumerate()
            {
                // candidate.starts_with(word) is always true here, so we just need candidate > word
                // which means candidate must be strictly longer (since it shares the prefix)
                if candidate.len() > word_len {
                    let score = score_extension_value(word_len, candidate.len());
                    links.push((prefix_start + idx, score));
                }
            }

            // --- Suffix matches: words that end with `word` ---
            // Use the reversed index: if a word ends with `word`, its reverse starts with reverse(`word`).
            let rev_word: String = word.chars().rev().collect();
            let suffix_start =
                reversed_index.partition_point(|(w, _)| w.as_str() < rev_word.as_str());
            let mut rev_upper_bytes = rev_word.as_bytes().to_vec();
            if let Some(last) = rev_upper_bytes.last_mut() {
                *last += 1;
            }
            let rev_upper_bound = unsafe { String::from_utf8_unchecked(rev_upper_bytes) };
            let suffix_end =
                reversed_index.partition_point(|(w, _)| w.as_str() < rev_upper_bound.as_str());

            for &(_, orig_idx) in &reversed_index[suffix_start..suffix_end] {
                let candidate = &final_wordlist_vec[orig_idx];
                // Must be longer (not the word itself), must not already be counted as a prefix match,
                // and must be lexicographically greater (matching the original score_extension behavior)
                if candidate.len() > word_len
                    && candidate.as_str() > word.as_str()
                    && !candidate.starts_with(word.as_str())
                {
                    let score = score_extension_value(word_len, candidate.len());
                    links.push((orig_idx, score));
                }
            }

            let substring_score: usize = links.iter().map(|(_, score)| score).sum();

            for (idx, _) in links.into_iter() {
                _ = backprop_points
                    .entry(idx)
                    .or_default()
                    .add_assign(substring_score);
            }

            (
                word_idx,
                word,
                WordData {
                    substring_score,
                    frequency,
                    objectionable: objectionable.contains(word),
                },
            )
        })
        .collect();

    // Sort by word index to maintain alphabetical order
    scored_word_list.sort_unstable_by_key(|(idx, _, _)| *idx);

    println!("Backpropagating word substring scores");
    for (word_idx, _, data) in scored_word_list.iter_mut() {
        if let Some(pts) = backprop_points.get(word_idx) {
            data.substring_score += *pts;
        }
    }

    println!("Formatting the output file");
    let word_list: Vec<String> = scored_word_list
        .into_iter()
        .map(
            |(
                _,
                word,
                WordData {
                    substring_score,
                    frequency,
                    objectionable,
                },
            )| {
                format!(
                    "{}{word} {substring_score} {frequency:.4}",
                    if objectionable { "*" } else { "" }
                )
            },
        )
        .collect();

    println!("Writing output file");

    let output_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("final_wordlist.txt");
    let output_file_contents = word_list.join("\n");

    fs::write(output_file_path, output_file_contents).expect("Output file should be writable");
}
