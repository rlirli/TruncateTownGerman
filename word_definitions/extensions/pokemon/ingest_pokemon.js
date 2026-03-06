/**
 * Fetches German Pokémon names and Pokédex entries for Gen 1+2 (IDs 1–251)
 * from PokéAPI to amend word_definitions/ and dict_builder/ output.
 *
 * Outputs:
 *   - output_pokemon_wordlist.txt: cleaned names for dict_builder/support_data/tranche_pokemon_add.txt
 *   - Inserts definitions into local_defs.db
 *
 * Usage: node ingest_pokemon.js
 */

const fs = require('fs');
const path = require('path');
const sqlite3 = require('sqlite3').verbose();
const zlib = require('zlib');

const MAX_ID = 251;
const BATCH_SIZE = 20; // concurrent fetches to be polite to the API
const LOCAL_DB_PATH = path.join(__dirname, '../../local_defs.db');
const RELEASE_DB_GZ_PATH = path.join(__dirname, '../../defs.db.gz');
const RELEASE_DB_PATH = path.join(__dirname, '../../defs.db');
const NAMES_OUTPUT = path.join(__dirname, 'output_pokemon_wordlist.txt');
const TRANCHE_OUTPUT = path.join(__dirname, '../../../dict_builder/support_data/tranche_pokemon_add.txt');

const RELEASE_MODE = process.argv.includes('--release');

const cleanGermanWord = (word) => {
    return word.toLowerCase()
        .replace(/ä/g, 'ae')
        .replace(/ö/g, 'oe')
        .replace(/ü/g, 'ue')
        .replace(/ß/g, 'ss');
};

const isValidWord = (cleaned) => {
    return cleaned.length >= 2 && /^[a-z]+$/.test(cleaned);
};

async function fetchSpecies(id) {
    const url = `https://pokeapi.co/api/v2/pokemon-species/${id}/`;
    const res = await fetch(url);
    if (!res.ok) throw new Error(`Failed to fetch species ${id}: ${res.status}`);
    return res.json();
}

async function fetchPokemon(id) {
    const url = `https://pokeapi.co/api/v2/pokemon/${id}/`;
    const res = await fetch(url);
    if (!res.ok) throw new Error(`Failed to fetch pokemon ${id}: ${res.status}`);
    return res.json();
}

function getGermanName(speciesData) {
    const entry = speciesData.names.find(n => n.language.name === 'de');
    return entry ? entry.name : null;
}

function getGermanFlavorText(speciesData) {
    // Get all German flavor texts, prefer newer games
    const deEntries = speciesData.flavor_text_entries
        .filter(e => e.language.name === 'de');
    if (deEntries.length === 0) return null;
    // Take the last (newest) entry and clean up whitespace
    const text = deEntries[deEntries.length - 1].flavor_text
        .replace(/\n/g, ' ')
        .replace(/\f/g, ' ')
        .replace(/\s+/g, ' ')
        .trim();
    return text;
}

function getGermanGenus(speciesData) {
    const entry = speciesData.genera.find(g => g.language.name === 'de');
    return entry ? entry.genus : null;
}

function getGeneration(id) {
    return id <= 151 ? 1 : 2;
}

function getGermanTypeName(typeName) {
    const typeMap = {
        'normal': 'Normal',
        'fire': 'Feuer',
        'water': 'Wasser',
        'grass': 'Pflanze',
        'electric': 'Elektro',
        'ice': 'Eis',
        'fighting': 'Kampf',
        'poison': 'Gift',
        'ground': 'Boden',
        'flying': 'Flug',
        'psychic': 'Psycho',
        'bug': 'Käfer',
        'rock': 'Gestein',
        'ghost': 'Geist',
        'dark': 'Unlicht',
        'dragon': 'Drache',
        'steel': 'Stahl',
        'fairy': 'Fee',
    };
    return typeMap[typeName] || typeName;
}

async function insertIntoDb(dbPath, pokemonList) {
    console.log(`\nInserting definitions into ${dbPath}...`);

    if (!fs.existsSync(dbPath)) {
        console.error(`ERROR: ${dbPath} does not exist.`);
        process.exit(1);
    }

    const db = new sqlite3.Database(dbPath);

    await new Promise((resolve, reject) => {
        db.serialize(() => {
            const stmt = db.prepare(`INSERT OR REPLACE INTO words (word, definitions) VALUES (?, ?)`);

            for (const p of pokemonList) {
                const defJson = JSON.stringify([{
                    word: p.cleanedName,
                    pos: "noun",
                    defs: [p.definition],
                    tags: ["Pokémon"],
                    roots: [],
                    forms: [],
                    objectionable: false,
                }]);
                stmt.run(p.cleanedName, defJson);
            }

            stmt.finalize((err) => {
                if (err) reject(err);
                else resolve();
            });
        });
    });

    db.close();
    console.log(`Inserted ${pokemonList.length} definitions into ${dbPath}`);
}

async function main() {
    console.log(`Fetching Pokémon data for Gen 1+2 (IDs 1–${MAX_ID})...\n`);

    const results = [];

    // Fetch in batches
    for (let start = 1; start <= MAX_ID; start += BATCH_SIZE) {
        const end = Math.min(start + BATCH_SIZE - 1, MAX_ID);
        const ids = [];
        for (let i = start; i <= end; i++) ids.push(i);

        const batchResults = await Promise.all(
            ids.map(async (id) => {
                try {
                    const [species, pokemon] = await Promise.all([
                        fetchSpecies(id),
                        fetchPokemon(id),
                    ]);
                    return { id, species, pokemon };
                } catch (err) {
                    console.error(`  Error fetching #${id}: ${err.message}`);
                    return null;
                }
            })
        );

        for (const r of batchResults) {
            if (r) results.push(r);
        }

        console.log(`  Fetched #${start}–#${end} (${results.length} total)`);
    }

    console.log(`\nProcessing ${results.length} Pokémon...\n`);

    const validPokemon = [];
    const skippedPokemon = [];

    for (const { id, species, pokemon } of results) {
        const germanName = getGermanName(species);
        if (!germanName) {
            skippedPokemon.push({ id, reason: 'no German name' });
            continue;
        }

        const cleaned = cleanGermanWord(germanName);
        if (!isValidWord(cleaned)) {
            skippedPokemon.push({ id, name: germanName, cleaned, reason: 'invalid characters after cleaning' });
            continue;
        }

        const flavorText = getGermanFlavorText(species);
        const genus = getGermanGenus(species);
        const gen = getGeneration(id);
        const types = pokemon.types
            .sort((a, b) => a.slot - b.slot)
            .map(t => getGermanTypeName(t.type.name));

        // Build definition string
        const defParts = [];
        if (genus) {
            defParts.push(`${genus} (Gen ${gen}, #${id}).`);
        } else {
            defParts.push(`Pokémon der Generation ${gen} (#${id}).`);
        }
        defParts.push(`Typ: ${types.join('/')}.`);
        if (flavorText) {
            defParts.push(flavorText);
        }

        validPokemon.push({
            id,
            originalName: germanName,
            cleanedName: cleaned,
            definition: defParts.join(' '),
            types,
        });
    }

    console.log(`Valid: ${validPokemon.length}, Skipped: ${skippedPokemon.length}`);
    if (skippedPokemon.length > 0) {
        console.log(`\nSkipped Pokémon:`);
        for (const s of skippedPokemon) {
            console.log(`  #${s.id} ${s.name || '?'} → ${s.cleaned || '?'}: ${s.reason}`);
        }
    }

    // 1. Write output_pokemon_wordlist.txt to use in dict_builder/
    const nameLines = validPokemon.map(p => p.cleanedName);
    const namesContent = nameLines.join('\n') + '\n';
    fs.writeFileSync(NAMES_OUTPUT, namesContent);
    console.log(`\nWrote ${nameLines.length} names to ${NAMES_OUTPUT}`);

    // Copy to dict_builder only in release mode
    if (RELEASE_MODE) {
        fs.writeFileSync(TRANCHE_OUTPUT, namesContent);
        console.log(`Wrote ${nameLines.length} names to ${TRANCHE_OUTPUT}`);
    } else {
        console.log(`Skipping dict_builder output (use --release to update ${path.basename(TRANCHE_OUTPUT)})`);
    }

    // 2. Insert into local_defs.db
    await insertIntoDb(LOCAL_DB_PATH, validPokemon);

    // 3. Optionally insert into production defs.db.gz
    if (RELEASE_MODE) {
        console.log(`\n=== RELEASE MODE ===`);
        console.log(`Decompressing ${RELEASE_DB_GZ_PATH}...`);

        if (!fs.existsSync(RELEASE_DB_GZ_PATH)) {
            console.error(`ERROR: ${RELEASE_DB_GZ_PATH} does not exist. Run 'npm start' in word_definitions first!`);
            process.exit(1);
        }

        // Decompress defs.db.gz -> defs.db
        const compressed = fs.readFileSync(RELEASE_DB_GZ_PATH);
        const decompressed = zlib.gunzipSync(compressed);
        fs.writeFileSync(RELEASE_DB_PATH, decompressed);
        console.log(`Decompressed to ${RELEASE_DB_PATH}`);

        // Insert into decompressed defs.db
        await insertIntoDb(RELEASE_DB_PATH, validPokemon);

        // Re-gzip defs.db -> defs.db.gz
        console.log(`Re-compressing ${RELEASE_DB_PATH}...`);
        const updatedDb = fs.readFileSync(RELEASE_DB_PATH);
        const recompressed = zlib.gzipSync(updatedDb);
        fs.writeFileSync(RELEASE_DB_GZ_PATH, recompressed);
        console.log(`Wrote ${RELEASE_DB_GZ_PATH}`);

        // Clean up uncompressed defs.db
        fs.unlinkSync(RELEASE_DB_PATH);
        console.log(`Cleaned up ${RELEASE_DB_PATH}`);
    } else {
        console.log(`\nSkipping release DB update (use --release to update defs.db.gz)`);
    }

    // 4. Summary
    console.log(`\n=== Summary ===`);
    console.log(`Total fetched: ${results.length}`);
    console.log(`Valid names: ${validPokemon.length}`);
    console.log(`Skipped: ${skippedPokemon.length}`);
    console.log(`Release mode: ${RELEASE_MODE ? 'YES' : 'no'}`);
    console.log(`\nNext steps:`);
    if (!RELEASE_MODE) {
        console.log(`\t>  Run with --release to automatically update shared wordlists and database.`);
        console.log(`\t   (OR manually: copy ${path.basename(NAMES_OUTPUT)} into ${path.relative(__dirname, TRANCHE_OUTPUT)})`);
    }
    console.log(`\t> cd dict_builder && cargo run --release`);
}

main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
});
