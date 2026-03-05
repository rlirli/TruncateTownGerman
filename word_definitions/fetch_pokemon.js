/**
 * Fetches German Pokémon names and Pokédex entries for Gen 1+2 (IDs 1–251)
 * from PokéAPI.
 *
 * Outputs:
 *   - pokemon_names.txt: cleaned names for tranche_german_1_add.txt
 *   - Inserts definitions into local_defs.db
 *
 * Usage: node fetch_pokemon.js
 */

const fs = require('fs');
const path = require('path');
const sqlite3 = require('sqlite3').verbose();

const MAX_ID = 251;
const BATCH_SIZE = 20; // concurrent fetches to be polite to the API
const DB_PATH = path.join(__dirname, 'local_defs.db');
const NAMES_OUTPUT = path.join(__dirname, 'pokemon_names.txt');

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

    // 1. Write pokemon_names.txt
    const nameLines = validPokemon.map(p => p.cleanedName);
    fs.writeFileSync(NAMES_OUTPUT, nameLines.join('\n') + '\n');
    console.log(`\nWrote ${nameLines.length} names to ${NAMES_OUTPUT}`);

    // 2. Insert into local_defs.db
    console.log(`\nInserting definitions into ${DB_PATH}...`);

    if (!fs.existsSync(DB_PATH)) {
        console.error(`ERROR: ${DB_PATH} does not exist. Run 'npm start' in word_definitions first!`);
        process.exit(1);
    }

    const db = new sqlite3.Database(DB_PATH);

    await new Promise((resolve, reject) => {
        db.serialize(() => {
            const stmt = db.prepare(`INSERT OR REPLACE INTO words (word, definitions) VALUES (?, ?)`);

            for (const p of validPokemon) {
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
    console.log(`Inserted ${validPokemon.length} definitions into local_defs.db`);

    // 3. Summary
    console.log(`\n=== Summary ===`);
    console.log(`Total fetched: ${results.length}`);
    console.log(`Valid names: ${validPokemon.length}`);
    console.log(`Skipped: ${skippedPokemon.length}`);
    console.log(`\nNext steps:`);
    console.log(`  1. Copy names: cat ${NAMES_OUTPUT}`);
    console.log(`  2. Paste into dict_builder/support_data/tranche_german_1_add.txt`);
    console.log(`  3. Rebuild dict: cd dict_builder && cargo run`);
}

main().catch(err => {
    console.error('Fatal error:', err);
    process.exit(1);
});
