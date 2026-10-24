#!/usr/bin/env bash
# Run at the end of a Mac mini job, whatever its outcome: drops what the job made that the next one doesn't
# reuse (the extracted assets, fidelity traces and web builds), so the minis' disk only holds the target
# cache, which setup.sh caps.
set -euo pipefail
cd "${GITHUB_WORKSPACE:?}"
rm -rf assets target/fidelity target/web target/tmp
df -h . | tail -n 1
