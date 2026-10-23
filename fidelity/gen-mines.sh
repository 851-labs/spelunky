#!/bin/sh
# Writes a replay that enters the first Mines level with the given seed and stands still in it, for sweeping
# level generation (scrLevelGen, scrRoomGen, scrEntityGen, ...) across seeds with scripts/fidelity.
#   fidelity/gen-mines.sh <seed> [frames] > replay.txt
# As tools/oracle/examples/returning.txt, but skipping the intro: a returning player with zeroed stats (so the
# START door leads into rLevel), the attack key twice to skip the intro, then left into the START door. The
# level loads on frame ~565; the default 900 frames stand still in it for ~335 frames. To reach levels 2-4,
# give mines-route (crates/harness) a replay of 600 frames, which ends just after the level loads.
set -eu
seed="${1:?usage: gen-mines.sh <seed> [frames]}"
frames="${2:-900}"
cat <<REPLAY
# Mines level 1 with seed $seed, standing still (fidelity/gen-mines.sh $seed $frames).
seed $seed
frames $frames
highscore 9000000 "MONEY"
highscore 8000000 "TIME"
highscore 7000000 "KILLS"
highscore 6000000 "SAVES"
highscore 5000000 "PLAYS"
highscore 4000000 "WINS"
highscore 3000000 "DEATHS"
highscore 2100001 "TUNNEL1"
highscore 1200001 "TUNNEL2"
highscore 0 "MINIGAMES"
20 hold X 2
60 hold X 2
450 hold left 70
535 hold up 5
REPLAY
