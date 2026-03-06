use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, BufRead},
    ops::AddAssign,
    path::PathBuf,
};

use dashmap::DashMap;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

/// Words with a frequency count below this threshold are excluded from the dictionary.
const MIN_WORD_FREQUENCY: usize = 5;

const WORD_FREQUENCIES_PATH: &str = "support_data/de_full.txt";
const WORD_DEFINITIONS_PATH: &str = "../word_definitions/valid_german_words.txt";
const OBJECTIONABLE_PATH: &str = "../word_definitions/objectionable.json";

type WordFrequency = usize;

/// Normalize a German word to ASCII lowercase by replacing umlauts with digraphs.
fn normalize_german_umlauts(word: &str) -> String {
    word.to_lowercase()
        .replace("ä", "ae")
        .replace("ö", "oe")
        .replace("ü", "ue")
        .replace("ß", "ss")
}

/// Primary determiner for which words do and do not qualify for inclusion in Truncate's validity dictionary.
fn should_include_word(word: &String, word_frequency: WordFrequency) -> bool {
    // One-letter words in Truncate can be a surprise, exclude them.
    if word.len() < 2 {
        return false;
    }
    // Truncate is ASCII-only — this also helps cut out proper names and words with punctuation
    if !word.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    // Filter out words that are too rare in the frequency corpus
    if word_frequency < MIN_WORD_FREQUENCY {
        return false;
    }
    return true;
}

fn load_word_frequencies() -> BTreeMap<String, f32> {
    println!("Loading word frequencies from file");
    let frequency_file =
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORD_FREQUENCIES_PATH)).expect(
            &format!("{WORD_FREQUENCIES_PATH} file should exist. Run ./setup_data.sh first!"),
        );
    let frequency_lines = io::BufReader::new(frequency_file)
        .lines()
        .flatten()
        .collect::<Vec<_>>();

    let mut frequency_lookup: BTreeMap<String, f32> = BTreeMap::new();

    // Word frequencies are listed in order,
    // so we can just use enumerate() for the rankings
    let mut frequencies = frequency_lines
        .into_par_iter()
        .enumerate()
        .map(|(i, wf)| {
            let (word, _) = wf
                .split_once(' ')
                .expect("Word frequencies are well formed");
            (normalize_german_umlauts(word), i as f32)
        })
        .collect::<Vec<_>>();

    let total_words = frequencies.len() as f32;
    frequencies.par_iter_mut().for_each(|(_, v)| {
        *v = (total_words - *v) / total_words;
    });

    frequency_lookup.extend(frequencies);

    println!("Recalculating word frequency counts");

    frequency_lookup
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

fn score_extension(target: &String, larger_word: &String) -> Option<usize> {
    if larger_word <= target {
        return None;
    }
    if larger_word.starts_with(target) || larger_word.ends_with(target) {
        let diff = larger_word.len() - target.len();
        if diff >= 5 {
            return Some(1);
        } else {
            return Some((5 - diff).pow(2));
        }
    }
    None
}

fn main() {
    println!("Starting the dict builder");
    let frequency_lookup = load_word_frequencies();

    println!("Loading candidate wordlists");
    let candidate_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORD_FREQUENCIES_PATH);
    let candidate_file =
        File::open(&candidate_file_path).expect(&format!("{WORD_FREQUENCIES_PATH} should exist"));
    let candidate_lines = io::BufReader::new(candidate_file).lines().flatten();

    let mut candidate_word_list: BTreeSet<String> = BTreeSet::new();
    candidate_word_list.extend(candidate_lines.filter_map(|line| {
        let (word, count) = line.split_once(' ').expect("Word frequency well formed");
        let frequency: WordFrequency = count.parse().expect("Word frequency count is a number");
        let cleaned = normalize_german_umlauts(word);
        if should_include_word(&cleaned, frequency) {
            Some(cleaned)
        } else {
            None
        }
    }));

    // To help filter out less desired words, we require words to _also_ be in the list of German word definitions.
    let valid_words = load_valid_german_words();
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

    let backprop_points: DashMap<&String, usize> = DashMap::new();
    let objectionable = load_objectionable();

    let mut scored_word_list = final_wordlist
        .par_iter()
        .map(|word| {
            let frequency = frequency_lookup.get(*word).cloned().unwrap_or(0.0);
            let links: Vec<_> = final_wordlist
                .iter()
                .filter_map(|w| score_extension(*word, *w).map(|score| (w, score)))
                .collect();
            let substring_score: usize = links.iter().map(|(_, score)| score).sum();

            for (word, _) in links.into_iter() {
                _ = backprop_points
                    .entry(*word)
                    .or_default()
                    .add_assign(substring_score);
            }

            (
                *word,
                WordData {
                    substring_score,
                    frequency,
                    objectionable: objectionable.contains(word),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    println!("Backpropagating word substring scores");
    scored_word_list.iter_mut().for_each(|(word, data)| {
        if let Some(pts) = backprop_points.get(word) {
            data.substring_score += *pts;
        }
    });

    println!("Formatting the output file");
    let word_list = scored_word_list
        .into_iter()
        .map(
            |(
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
        .collect::<Vec<_>>();

    println!("Writing output file");

    let output_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("final_wordlist.txt");
    let output_file_contents = word_list.join("\n");

    fs::write(output_file_path, output_file_contents).expect("Output file should be writable");
}
