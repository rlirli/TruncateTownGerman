use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, BufRead},
    ops::AddAssign,
    path::PathBuf,
};

use dashmap::DashMap;
use rayon::iter::{
    IntoParallelRefIterator,
    ParallelIterator,
};

fn clean_german_word(word: &str) -> String {
    word.to_lowercase()
        .replace("ä", "ae")
        .replace("ö", "oe")
        .replace("ü", "ue")
        .replace("ß", "ss")
}

fn load_german_data() -> (BTreeMap<String, f32>, BTreeSet<String>) {
    println!("Loading words and frequencies from de_50k.txt");
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("support_data/de_50k.txt");
    let file = File::open(file_path).expect("de_50k.txt file should exist");
    
    let lines = io::BufReader::new(file).lines().flatten().collect::<Vec<_>>();
    
    let mut frequency_lookup = BTreeMap::new();
    let mut word_set = BTreeSet::new();
    
    let total_words = lines.len() as f32;
    for (i, line) in lines.into_iter().enumerate() {
        let (word, _) = line.split_once(' ').expect("Word frequency well formed");
        let cleaned = clean_german_word(word);
        
        if cleaned.chars().count() < 2 {
            continue;
        }
        
        if !cleaned.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }

        let freq = (total_words - i as f32) / total_words;
        frequency_lookup.insert(cleaned.clone(), freq);
        word_set.insert(cleaned);
    }
    
    (frequency_lookup, word_set)
}

fn load_additions() -> BTreeSet<String> {
    println!("Loading additional data from files");

    let files = [
        "support_data/tranche_german_1_add.txt"
    ]
    .map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("add files should exist")
    });

    let mut set = BTreeSet::new();
    for f in files {
        for line in io::BufReader::new(f).lines().flatten() {
            let cleaned = clean_german_word(&line);
            if cleaned.chars().count() >= 2 && cleaned.chars().all(|c| c.is_ascii_lowercase()) {
                set.insert(cleaned);
            }
        }
    }
    set
}

fn load_removals() -> BTreeSet<String> {
    println!("Loading removal data from files");

    let files = [
        "support_data/tranche_german_1_del.txt"
    ].map(|f| {
        File::open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(f))
            .expect("del files should exist")
    });

    let mut set = BTreeSet::new();
    for f in files {
        for line in io::BufReader::new(f).lines().flatten() {
            let cleaned = clean_german_word(&line);
            set.insert(cleaned);
        }
    }
    set
}

fn load_objectionable() -> Vec<String> {
    let input =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../word_definitions/objectionable.json"))
            .expect("../word_definitions/objectionable.json should exist");
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

fn load_valid_german_words() -> std::collections::HashSet<String> {
    println!("Loading valid German words from dictionary generation list");
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../word_definitions/valid_german_words.txt");
    
    let file = File::open(file_path).expect("valid_german_words.txt should exist. Make sure to run `npm start` in word_definitions first!");
    
    io::BufReader::new(file)
        .lines()
        .flatten()
        .map(|line| clean_german_word(&line))
        .filter(|w| w.chars().count() >= 2 && w.chars().all(|c| c.is_ascii_lowercase()))
        .collect()
}

fn main() {
    println!("Starting the dict builder");
    let (frequency_lookup, mut final_wordlist) = load_german_data();

    // Filter list using actual definitions
    let valid_words = load_valid_german_words();
    let initial_count = final_wordlist.len();
    final_wordlist.retain(|word| valid_words.contains(word));
    println!("Filtered out {} words lacking a real definition.", initial_count - final_wordlist.len());

    let additions = load_additions();
    final_wordlist.extend(additions.into_iter());

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

    // Convert BTreeSet to Vec for efficient parallel iteration
    let final_wordlist_vec: Vec<String> = final_wordlist.into_iter().collect();

    let backprop_points: DashMap<&String, usize> = DashMap::new();
    let objectionable: std::collections::HashSet<String> = load_objectionable().into_iter().collect();

    let mut scored_word_list = final_wordlist_vec
        .par_iter()
        .map(|word| {
            let frequency = frequency_lookup.get(word).cloned().unwrap_or(0.99);

            let links: Vec<_> = final_wordlist_vec
                .iter()
                .filter_map(|w| score_extension(word, w).map(|score| (w, score)))
                .collect();
            
            let substring_score: usize = links.iter().map(|(_, score)| score).sum();

            for (w, score) in links.into_iter() {
                _ = backprop_points
                    .entry(w)
                    .or_default()
                    .add_assign(score);
            }

            (
                word.clone(),
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
