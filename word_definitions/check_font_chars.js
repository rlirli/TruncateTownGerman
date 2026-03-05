/**
 * Checks whether the definitions in the SQLite database contain characters
 * that are not supported by the fonts used in the application.
 *
 * By default, decompresses and checks the release database (defs.db.gz).
 * Falls back to local_defs.db if defs.db.gz is not found.
 *
 * Usage:
 *   node check_font_chars.js              # uses defs.db.gz (preferred), then local_defs.db
 *   node check_font_chars.js my_defs.db   # use a specific db file
 */

const path = require('path');
const fs = require('fs');
const zlib = require('zlib');
const sqlite3 = require('sqlite3').verbose();
const opentype = require('opentype.js');

// ── Paths ──
const FONTS = [
    { name: 'm5x7.ttf', id: 'pixel', path: path.join(__dirname, '../truncate_client/font/m5x7.ttf') },
    { name: 'PressStart2P-Regular.ttf', id: 'heavy', path: path.join(__dirname, '../truncate_client/font/PressStart2P-Regular.ttf') }
];

/**
 * Extracts character set from a TTF using opentype.js
 */
function getFontCharSet(fontPath) {
    if (!fs.existsSync(fontPath)) {
        console.warn(`⚠️  Font not found: ${fontPath}`);
        return new Set();
    }
    try {
        const font = opentype.loadSync(fontPath);
        const chars = new Set();
        // Traverse all glyphs in the font
        for (let i = 0; i < font.glyphs.length; i++) {
            const glyph = font.glyphs.get(i);
            if (glyph.unicode) {
                chars.add(String.fromCodePoint(glyph.unicode));
            }
            if (glyph.unicodes) {
                for (const u of glyph.unicodes) {
                    chars.add(String.fromCodePoint(u));
                }
            }
        }
        return chars;
    } catch (e) {
        console.error(`Error extracting chars from ${fontPath}:`, e.message);
        return new Set();
    }
}

console.log('Loading character maps from fonts...');
for (const f of FONTS) {
    f.charSet = getFontCharSet(f.path);
    console.log(`- ${f.name} (${f.id}): ${f.charSet.size} glyphs`);
}
console.log();

// ── Resolve which database file to use ──
function decompressGz(gzPath) {
    const tmpPath = '/tmp/defs_check.db';
    console.log(`Decompressing ${path.basename(gzPath)} → ${tmpPath} …`);
    const compressed = fs.readFileSync(gzPath);
    const decompressed = zlib.gunzipSync(compressed);
    fs.writeFileSync(tmpPath, decompressed);
    console.log('Done.\n');
    return tmpPath;
}

// used for logging in summary table
let sourceDbLabel = '';

function resolveDbPath() {
    const explicit = process.argv[2];
    if (explicit) {
        const p = path.resolve(__dirname, explicit);
        if (!fs.existsSync(p)) {
            console.error(`File not found: ${p}`);
            process.exit(1);
        }
        sourceDbLabel = path.join(path.basename(path.dirname(p)), path.basename(p));
        if (p.endsWith('.gz')) return decompressGz(p);
        return p;
    }

    const gzDb = path.join(__dirname, 'defs.db.gz');
    if (fs.existsSync(gzDb)) {
        sourceDbLabel = path.join(path.basename(path.dirname(gzDb)), path.basename(gzDb));
        return decompressGz(gzDb);
    }

    const localDb = path.join(__dirname, 'local_defs.db');
    if (fs.existsSync(localDb)) {
        console.log('defs.db.gz not found, falling back to local_defs.db\n');
        sourceDbLabel = path.join(path.basename(path.dirname(localDb)), path.basename(localDb));
        return localDb;
    }

    console.error('No database found.');
    process.exit(1);
}

const dbPath = resolveDbPath();
console.log(`Using database: ${dbPath}\n`);

const db = new sqlite3.Database(dbPath, sqlite3.OPEN_READONLY);

// ── Scan ──
const MAX_EXAMPLES = 5;

// Initialize missing character tracking per font
for (const f of FONTS) {
    f.missingChars = new Map(); // char → { count, codepoint, examples: Set }
}

// ── Core Character Set ──
const CORE_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!?()-+←→ÄäÖöÜüß,.:;";

db.each(
    `SELECT word, definitions FROM words`,
    (err, row) => {
        if (err) return;
        let entries;
        try {
            entries = JSON.parse(row.definitions);
        } catch {
            return;
        }

        for (const entry of entries) {
            // The game (battle.rs) specifically only renders the first definition,
            // optionally prepended by the part_of_speech.
            const pos = entry.pos || '';
            const defText = entry.defs && entry.defs.length > 0 ? entry.defs[0] : '';
            const renderedString = pos ? `${pos}: ${defText}` : defText;

            for (const ch of renderedString) {
                const codepoint = ch.codePointAt(0);
                if (codepoint <= 31) continue; // Skip control characters

                for (const f of FONTS) {
                    if (!f.charSet.has(ch)) {
                        if (!f.missingChars.has(ch)) {
                            f.missingChars.set(ch, { count: 0, codepoint, examples: new Set() });
                        }
                        const info = f.missingChars.get(ch);
                        info.count++;
                        if (info.examples.size < MAX_EXAMPLES) {
                            info.examples.add(row.word);
                        }
                    }
                }
            }
        }
    },
    (err, totalRows) => {
        if (err) {
            console.error('Query error:', err.message);
            process.exit(1);
        }

        console.log(`Scanned ${totalRows} word rows.\n`);

        for (const f of FONTS) {
            console.log(`====================================================================`);
            console.log(`  Font Analysis: ${f.name}`);
            console.log(`====================================================================`);

            if (f.missingChars.size === 0) {
                console.log(`✅ All characters in definitions are supported by ${f.name}.\n`);
            } else {
                const sorted = [...f.missingChars.entries()].sort((a, b) => b[1].count - a[1].count);

                console.log(`⚠️  Found ${sorted.length} character(s) NOT supported by ${f.name}:\n`);
                console.log(`${'Char'.padEnd(6)} ${'U+Code'.padEnd(8)} ${'Count'.padEnd(10)} Examples`);
                console.log(`${'─'.repeat(6)} ${'─'.repeat(8)} ${'─'.repeat(10)} ${'─'.repeat(40)}`);

                for (const [ch, info] of sorted) {
                    const displayChar = ch === '\n' ? '\\n' : ch === '\t' ? '\\t' : ch === '\r' ? '\\r' : ch;
                    const codepoint = `U+${info.codepoint.toString(16).toUpperCase().padStart(4, '0')}`;
                    const examples = [...info.examples].join(', ');
                    console.log(`${displayChar.padEnd(6)} ${codepoint.padEnd(8)} ${String(info.count).padEnd(10)} ${examples}`);
                }
                console.log("\n");
            }
        }
        
        console.log(`====================================================================`);
        console.log(`  Overall Summary`);
        console.log(`  Analysed all word definitions in ${sourceDbLabel}`);
        console.log(`  to verify glyph support in game fonts.`);
        console.log(`====================================================================`);
        console.log(`${'Font'.padEnd(25)} | ${'Char mis'.padEnd(8)} | ${'Core'.padEnd(10)} | Missing Characters`);
        console.log(`${'─'.repeat(25)} | ${'─'.repeat(8)} | ${'─'.repeat(10)} | ${'─'.repeat(30)}`);
        
        for (const f of FONTS) {
            const missingStr = [...f.missingChars.keys()]
                .filter(ch => ch.trim().length > 0 && ch.codePointAt(0) > 31)
                .join('');
            
            // Check missing from CORE_CHARS
            const missingCore = [];
            for (const ch of CORE_CHARS) {
                if (!f.charSet.has(ch)) {
                    missingCore.push(ch);
                }
            }
            const coreStatus = missingCore.length === 0 ? '✅' : `${missingCore.length} missing (${missingCore.join('')})`;

            console.log(`${f.name.padEnd(25)} | ${String(f.missingChars.size).padEnd(8)} | ${coreStatus.padEnd(10)} | ${missingStr}`);
        }
        console.log(`\n\n`);

        db.close();
    }
);

