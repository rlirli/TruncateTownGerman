# TruncateTownGerman - Git Reorganization Plan

## Todo Before Publishing

- **Educate me about env best practice**: I'm familiar with .env files of course. But not about where to manage it. There's of course the VPS itself which i can SSH into. Then GitHub Actins env variables. Then those in docker-compose.yml. (docker-compose.yml doesn't necessarily need to be inside the repository itself. It needs to be copied to the VPS anyways, since the github action performs: `command="cd /opt/containers/game-truncate && docker compose pull && docker compose up -d` from `/home/github_deploy/.ssh/authorized_keys`) -- There might certainly be other best practices for env management in my scenario.
- **List all env vars**: List all env vars (and things that should be) and their purpose and when they are required (runtime, build, build of WHAT?).
- **Sanitize Environment Variables**: Extract production URLs from `web_client/src/_data/env.js` and `web_client/src/_includes/page.html`. Use `.env` substitution in the frontend pipeline so `truncate.igbekele.de` is not hardcoded.
- **Sanitize `docker-compose.yaml`**: The current file hardcodes `SIGNING_SECRET` and Postgres credentials. Move these into a ???`.env` file??? and use interpolation (e.g. `SIGNING_SECRET=${SIGNING_SECRET}`).
- **Update READMEs**
- **Virtual Keyboard Shift**: Start dev client locally exposed on local network so i can inspect keyboard shift better on my phone.

## Proposed Commit Split (WIP)

```text
feat(client): Enhance virtual keyboard layout shift behavior on mobile

Improves interaction by separating layout space for keyboards and
directly entering typing mode when the dictionary is opened.
```

- **Files affected**: `truncate_client/src/regions/active_game/control_strip.rs`
- **Note**: This is highly upstreamable and should be isolated.

```text
fix(client): Resolve WebSocket reconnect token loss

The client now correctly retains login context across reconnections
to prevent hanging on definition requests.
```

- **Files affected**: `truncate_client/src/web_comms.rs`
- **Note**: Upstreamable fix, isolate strictly to networking.

```text
perf(dict_builder): Optimize dictionary builder performance

Introduced parallel processing, DashMap, and binary searching
via partition_point to drop build times exponentially.
```

- **Files affected**: `dict_builder/src/main.rs` (ONLY the performance optimizations, keep the english/german specific path changes separated if possible)

```text
ci: optimize Docker image builds and setup Docker Compose infrastructure
```

- **Files affected**: `Dockerfile.client`, `Dockerfile.citadel`, `docker-compose.yaml`.
- **Note**: Should contain the _clean_ (non-hardcoded) compose file.

```text
ci: configure continuous deployment workflows
```

- **Files affected**: `.github/workflows/deploy-citadel.yml`, `.github/workflows/deploy-client.yml`, `.github/workflows/deploy.yml`, `.github/workflows/outpost.yml`, `.github/workflows/release.yml`

```text
chore(dict): script German frequency and rank data acquisition
```

- **Files affected**: `dict_builder/setup_data.sh`, `.gitignore` (add rules for data artifacts like `*.7z`, `*.csv`, `de_full.txt`)

```text
feat(core): adapt letter bag frequencies and values for German language
```

- **Files affected**: `truncate_core/src/bag.rs`

```text
feat(dict): create font character validation utility
```

- **Files affected**: `word_definitions/check_font_chars.js`

```text
feat(ui): replace primary font to support German umlauts
```

- **Files affected**: `truncate_client/font/m5x7.ttf`, `web_client/src/_data/credits.js` (Only the font attribution piece)

```text
feat(dict): adapt Wiktextract definition processor for German data
```

- **Files affected**: `word_definitions/build_sq.js`

```text
feat(dict): adapt dictionary builder rules for German words
```

- **Files affected**: `dict_builder/src/main.rs` (The path changes, German rules logic), `dict_builder/README.md`

```text
feat(data): extend definitions with Pokémon Generation 1+2
```

- **Files affected**: `word_definitions/extensions/pokemon/ingest_pokemon.js`, `word_definitions/extensions/pokemon/output_pokemon_wordlist.txt`, `word_definitions/extensions/pokemon/license.txt`, `word_definitions/package.json` (scripts related to this), `dict_builder/src/main.rs` (addition to `ADDITION_TRANCHES_PATHS`)

```text
feat(data): apply initial German inclusion/exclusion tranches
```

- **Files affected**: `dict_builder/support_data/tranche_1_add.txt` (deleted), `dict_builder/support_data/tranche_2_add.txt` (deleted), `dict_builder/support_data/tranche_3_add.txt` (deleted), `dict_builder/support_data/tranche_3_del.txt` (deleted), `dict_builder/support_data/tranche_german_1_add.txt` (created/kept empty), `dict_builder/support_data/tranche_german_1_del.txt` (created)

```text
build: generate final German wordlist and definition database
```

- **Files affected**: `dict_builder/final_wordlist.txt`, `word_definitions/defs.db.gz`

```text
feat(ui): update credits and dynamically bind environment URL
```

- **Files affected**: `web_client/src/_data/credits.js` (Remaining attribution lines), `web_client/src/_includes/page.html` (Environment variable binding instead of hard-coded server)

```text
fix: miscellaneous upstream connection and routing changes
```

- **Files affected**: `truncate_server/src/main.rs`, `truncate_client/src/app_outer.rs`, `truncate_client/src/lib.rs`, `truncate_client/src/main.rs` (Assuming these are related to minor URL bindings in the client routing or auth, based on standard patterns in the repo)

## Current Diff Stat vs origin/main

```text
 .github/workflows/deploy-citadel.yml               |     57 +
 .github/workflows/deploy-client.yml                |     57 +
 .github/workflows/deploy.yml                       |     79 +
 .github/workflows/outpost.yml                      |      2 +-
 .github/workflows/release.yml                      |      2 +-
 .gitignore                                         |      8 +
 Dockerfile.citadel                                 |      2 +-
 Dockerfile.client                                  |    118 +-
 dict_builder/README.md                             |     60 +-
 dict_builder/final_wordlist.txt                    | 363829 ++++++++++++------
 dict_builder/setup_data.sh                         |     95 +
 dict_builder/src/main.rs                           |    438 +-
 dict_builder/support_data/tranche_1_add.txt        |     48 -
 dict_builder/support_data/tranche_2_add.txt        |   3787 -
 dict_builder/support_data/tranche_3_add.txt        |      2 -
 dict_builder/support_data/tranche_3_del.txt        |    731 -
 dict_builder/support_data/tranche_german_1_add.txt |      0
 dict_builder/support_data/tranche_german_1_del.txt |      6 +
 docker-compose.yaml                                |     65 +
 truncate_client/font/m5x7.ttf                      |    Bin 0 -> 34300 bytes
 truncate_client/src/app_outer.rs                   |      2 +-
 truncate_client/src/lib.rs                         |      2 +-
 truncate_client/src/main.rs                        |      2 +-
 .../src/regions/active_game/control_strip.rs       |     32 +-
 truncate_client/src/web_comms.rs                   |      9 +
 truncate_core/src/bag.rs                           |    140 +-
 truncate_server/src/main.rs                        |      6 +-
 web_client/src/_data/credits.js                    |     42 +-
 web_client/src/_includes/page.html                 |      2 +-
 word_definitions/build_sq.js                       |    106 +-
 word_definitions/check_font_chars.js               |    220 +
 word_definitions/defs.db.gz                        |    Bin 30563141 -> 68075465 bytes
 .../extensions/pokemon/ingest_pokemon.js           |    291 +
 word_definitions/extensions/pokemon/license.txt    |      5 +
 .../extensions/pokemon/output_pokemon_wordlist.txt |    247 +
 word_definitions/package.json                      |      7 +-
 36 files changed, 252802 insertions(+), 117697 deletions(-)
```

## Raw Commit History (origin/main..HEAD)

```text
refactor dict_builder tranches

chore: Reorganize word_definiions/extensions pokemono files add license

fix(dict_builder): Undo debug-only changes
from earlier commit 95a1f2d99d36456dc9697628997d9c4117af3230 [95a1f2d]

chore: Undo unnecessary changes

pref(dict_builder): Filter out min_frequency earlier

feat: Add a Jupyter notebook for exploring word frequency thresholds and rename `valid_german_words.txt` to `words_with_definitions.txt` across the codebase.

feat(dict_builder): Add third frequency list dewiki

feat(dict_builder): Add second frequency list decow

feat: automate Pokémon data fetching in release mode and update post-processing instructions

feat(word_definitions): Improve sqlite DB Insert performance

dict_builder: Performance optimizations applied

The main bottleneck was the O(N²) scoring loop — with ~128K words in the final wordlist, every word was being compared against every other word (~16.5 billion comparisons). Here's what I changed:

🚀 Major: Binary Search for Prefix/Suffix Matching (O(N²) → ~O(N log N))
Instead of scanning all N words for each word:

Prefix matches: Since the word list is sorted, all words starting with a given word form a contiguous range. We use partition_point() (binary search) to find this range in O(log N).
Suffix matches: Pre-built a reversed-word index. To find words ending with "tion", we search for reversed words starting with "noit" — again using binary search.
🔄 Moderate: Eliminated Double File Read
Previously de_full.txt (17MB) was read and parsed twice — once for frequencies, once for candidates. Combined into a single
load_frequencies_and_candidates() function.

📦 Moderate: Vec-based Iteration Restored
The post-commit code iterated over BTreeSet<&String> (tree node pointer chasing). Restored converting to Vec<String> for cache-friendly sequential memory access during the scoring loop.

⚡ Minor: HashSet for Objectionable Lookups
Post-commit code used Vec::contains() (O(n)) for objectionable word checks. Restored HashSet<String> for O(1) lookups.

Behavioral Parity
All scoring semantics are preserved, including the lexicographic ordering guard (larger_word > target) on suffix matches. The output should be identical to the current version.

feat: Remove abbreviations from dict and definitions

docs(dict_builder): Amend readme

refactor: Revert to our preiovus

feat: directly set ui_state.dictionary_focused=true on button click
Earlier commit 97d35583b8d43a066b950231b2aa727fdca2da20 only added dictionary_opened_by_keyboard. But that did not auto-open keyboard on iOS. Hopefully this works.

feat: clarify an existing Wiktextract credit and add new credits for German Wiktionary and FrequencyWords.

feat: greatly expand German wordlist by processing new full German support data.

feat: directly enter typing mode when opening the dictionary.

refactor: Extract virtual keyboard offset calculation into a dedicated function

fix: Apply keyboard offset only on mobile devices with touch input.

feat: add script to verify font character support in database definitions.

feat: Replace at01 with m5x7 font
To support Umlaut characters

feat: Add gen 1+2 pokemon names to release word_definitions

feat: Add gen 1+2 pokemon names to dict_builder and local_defs.db

fix: Add german umlauts to font assets

feat: Re-run with German word definitions

feat: Update letter frequencies for actual German dict
TILE_GENERATIONS

ci: Revert Dockerfile.citadel

build: Add libsqlite3-dev as a build dependency to Dockerfile.citadel.

ci: update Node.js to version 24 in Dockerfile.client

ci: enable Buildx with docker-container driver for GHA cache export
- Adds setup-buildx-action step with docker-container driver
- Required so cache-to: type=gha works without "docker driver cannot export" errors
- Keeps cross-run layer caching functional for client & citadel builds

build: Add commit SHA as an additional Docker image tag in deployment workflows.

build: make builds deterministic + actually cache Rust deps in CI
- Pin Rust version (1.93.1) instead of `latest` for reproducible builds
- Reorder Dockerfiles to copy Cargo.toml/Cargo.lock first
- Add dummy pre-build step to cache dependency compilation separately
- Move TR_COMMIT / TR_MSG / TR_ENV args to final layer so commit changes don’t invalidate dependency cache
- Clean apt layers to reduce image size
- Enable BuildKit GHA cache (cache-from/cache-to type=gha)

Result:
- Rust dependencies no longer rebuild on every source change
- Commit hash no longer forces full rebuild
- CI builds drop from “recompile the universe” to incremental
- Deterministic toolchain instead of floating compiler updates

fix: resolve WebSocket reconnect token loss and database connection timeouts
The client now correctly retains login context across reconnections to prevent hanging on definition requests. Additionally, the server now proactively validates idle database connections before use to prevent silent query failures.

fix: Add SIGNING_SECRET env var

fix: Set network traefik label in docker compose
Fixes network between citadel and client

fix: Hopefully rise tiles above virtualkeyboard

fix: Also convert german umlauts in word_definitions

feat: Add "db". Revert auth workaround
Reverts auth workaround from commit 2ec188896f151ab2f68047b0ef2d7d08c5ad099c
With the workaround re-visiting user causes:
`truncate-citadel  | Player tried to login with a bad token and failed ! ! ! ! ! ! ! ! ! ! !`
resulting in connection refused between front- and backend.

feat: Add german word_definitions/ and pop words without from final_wordlist

feat: Replace tranche files

fix: Skip postgres in auth

fix: Specify SSH Port 2222

ops: Add GitHub Workflow deploy script

Customize Deploy URLS

correct fix for <2 letter words (and other stuff i didnt ask for)

... bad fix for 1-letter word word and bad NPC

Revert "dunno"
This reverts commit a06735bb19772d914cc8d8411de12e81ca13862f.

dunno

feat: german
```
