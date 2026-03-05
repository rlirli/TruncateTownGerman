const path = require('path');
const fs = require('fs');
const readline = require('readline');
const sqlite3 = require('sqlite3').verbose();
const zlib = require('zlib');

const input_file = path.join(__dirname, "kaikki.org-dictionary-German-DE.jsonl.gz");

if (!fs.existsSync(input_file)) {
    console.error(`Need to build word definitions from a dictionary reference.`);
    console.error(`Download the German JSON data from https://kaikki.org/dictionary/German/`);
    console.error(`And place the file at ${input_file}`);
    process.exit(1);
}

const rl = readline.createInterface({
    input: fs.createReadStream(input_file).pipe(zlib.createGunzip()),
    crlfDelay: Infinity
});

let written = 0;
let skipped = 0;
let writable = 0;

const words = {};

const objectionable_tags = [
    "vulgar",
    "offensive",
    "unpleasant",
    "objectionable",
    "derogatory",
    "genitalia",
    "sex",
    "sexual intercourse",
    "fascist",
    "racist",
    "anti-Semitic",
    "xenophobic",
    "supremacist",
    "ultranationalist",
    "slur",
];

const cleanGermanWord = (word) => {
    return word.toLowerCase()
        .replace(/ä/g, 'ae')
        .replace(/ö/g, 'oe')
        .replace(/ü/g, 'ue')
        .replace(/ß/g, 'ss');
};

const writeWord = (word_json) => {
    const cleaned_word = cleanGermanWord(word_json.word);

    // Skip words with whitespace, or punctuation
    if (/[^a-zA-Z]/.test(cleaned_word)) {
        skipped += 1;
        return;
    };

    const out_obj = {
        word: cleaned_word,
        pos: word_json.pos,
        defs: word_json.senses.flatMap(sense => sense.raw_glosses || sense.glosses || []),
        tags: word_json.senses.flatMap(sense => [...(sense.tags ?? []), ...(sense.links ?? []).map(link => link[0])]),
        roots: word_json.senses.flatMap(sense => [...(sense.form_of ?? []).map(form => cleanGermanWord(form.word))]),
        forms: (word_json.forms ?? []).flatMap(form => cleanGermanWord(form.form)),
        objectionable: false,
    };

    for (const tag of out_obj.tags) {
        for (const objectionable_tag of objectionable_tags) {
            if (tag.includes(objectionable_tag)) {
                out_obj.objectionable = true;
            }
        }
    }

    if (!out_obj.defs.length) {
        out_obj.defs.push(out_obj.etymology_text || "No definition found");
    }
    if (out_obj.defs.some(def => !def)) {
        console.error(`Bad def for ${word_json.word}:`);
        console.error(out_obj);
        console.error(JSON.stringify(word_json));
        process.exit(1);
    } else if (out_obj.defs.length === 0) {
        console.warn(`- - - - - - - - - - -`);
        console.warn(`No defs for ${word_json.word}`);
        console.warn(JSON.stringify(word_json, null, 2));
        console.warn(`- - - - - - - - - - -`);
    }

    const word_key = `${out_obj.word}_tr`; // Fixes clash with `constructor`
    words[word_key] = words[word_key] || [];
    words[word_key].push(out_obj);

    written += 1;
    if (written % 5000 === 0) {
        console.log(`• Processed: ${written}, Skipped: ${skipped}`);
    }
}

rl.on('line', (line) => {
    writable += 1;
    try {
        const data = JSON.parse(line);
        writeWord(data);
    } catch (e) {
        // Skip erroneous lines (e.g. malformed JSON) instead of crashing
        skipped += 1;
    }
});

rl.on('close', () => {
    console.log(`\n-------------\n`);

    console.log(`• Mapping objectionable words`);
    const objectionable_words = [];

    for (const word_data of Object.values(words)) {
        for (const word_datum of word_data) {
            let objectionable = word_datum.objectionable;

            for (const root of word_datum.roots) {
                if (words[`${root}_tr`]?.objectionable) {
                    objectionable = true;
                }
            }

            if (objectionable) {
                if (!objectionable_words.includes(word_datum.word)) {
                    objectionable_words.push(word_datum.word);
                }

                for (const form of word_datum.forms) {
                    if (!objectionable_words.includes(form)) {
                        objectionable_words.push(form);
                    }
                }
            }
        }
    }

    console.log(`• Writing objectionable words`);
    fs.writeFileSync(`objectionable.json`, JSON.stringify(objectionable_words, null, 2));

    console.log(`• Sorting words`);
    const keys = Object.keys(words).sort();

    console.log(`• Writing valid_german_words.txt`);
    const valid_words = keys.map(k => k.replace(/_tr$/, ''));
    fs.writeFileSync(`valid_german_words.txt`, valid_words.join('\n'));

    console.log(`• Writing words`);
    const output_db = new sqlite3.Database('defs.db');
    output_db.serialize(() => {
        // Create a table for words
        output_db.run(`CREATE TABLE words (
            word TEXT,
            definitions TEXT
        )`);

        output_db.run(`CREATE INDEX index_word ON words (word)`);

        for (const key of keys) {
            output_db.run(`INSERT INTO words (word, definitions) VALUES (?, ?)`, [key.replace(/_tr$/, ''), JSON.stringify(words[key])], function (err) {
                if (err) {
                    return console.log(err.message);
                }
            });
        }
    });
    output_db.close(() => {
        console.log(`\n-------------\n`);

        console.log(`• Ingested ${writable} words`);
        console.log(`• Processed ${written} words`);
        console.log(`• Skipped ${skipped} words`);
        console.log(`• Total processed ${written + skipped} words`);
        console.log(`• Total output ${keys.length} words`);
        if (written + skipped !== writable) {
            console.error(`ERR: Didn't process all words.`);
            process.exit(1);
        }
    });
});
