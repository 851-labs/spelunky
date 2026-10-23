# inventory-py

Throwaway Python tools that parsed `spelunky.gmk` and produced `docs/source-inventory.md` (851-3117).
Superseded by the Rust `gmk` crate and extractor (851-3122); kept so the inventory can be regenerated.

The file paths come from arguments. Run each tool with `python3 -I`.

- `gmk_parse.py <spelunky.gmk> <out.pkl>`: parses the GM 8.0 `.gmk` (follows LateralGM's `GmFileReader.java`).
- `dump.py <out.pkl> <out-dir>`: writes scripts, object events, rooms and resource metadata as files.
- `analyze.py <out.pkl> <gmlkeywords.properties> <gmlfunctions.txt> <analysis.json>`: tokenizes GML and counts
  built-in function and variable use. The two reference lists come from JoshEdit (used by LateralGM).
- `tokgrep.py <out.pkl> <regex> [ctx]`: token-aware search over code only (skips comments and strings).
- `summarize.py <out.pkl> <analysis.json> <...> <narrative.md> <SUMMARY.md>`: builds the inventory markdown.
