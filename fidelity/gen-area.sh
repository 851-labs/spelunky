#!/bin/sh
# Writes a replay that enters the first level of an area with the given seed and stands still in it, for
# sweeping level generation (scrLevelGen, scrRoomGen*, scrEntityGen, ...) across seeds with scripts/fidelity.
#   fidelity/gen-area.sh <mines|jungle|ice> <seed> [frames] > replay.txt
# All skip the intro with the attack key twice.
# - mines: as tools/oracle/examples/returning.txt, a returning player with zeroed stats (so the START door leads
#   into rLevel), then left into the START door. Level 1 loads on frame ~565.
# - jungle: as fidelity/replays/title-shortcut5.txt, every shortcut unlocked (TUNNEL1 and TUNNEL2 at their
#   offsets, $100 of MONEY), then up the ladder to the shortcut gallery and into the level 5 door. Level 5
#   loads on frame ~781.
# - ice: the same table, then into the level 9 door (oXShortcut9, x 144): the player stops on it after jumping
#   off the ladder. Level 9 (room rLevel2) loads on frame 760.
# The default frames stand still in the level for ~335 frames. To reach the area's next levels, give
# area-route (crates/harness) a replay that ends just after the level loads: 600 frames for mines, 800 for
# jungle, 770 for ice.
set -eu
usage="usage: gen-area.sh <mines|jungle|ice> <seed> [frames]"
area="${1:?$usage}"
seed="${2:?$usage}"
case "$area" in
mines) frames="${3:-900}" money=9000000 tunnel1=2100001 tunnel2=1200001 ;;
jungle) frames="${3:-1116}" money=9000100 tunnel1=2000000 tunnel2=1000000 ;;
ice) frames="${3:-1095}" money=9000100 tunnel1=2000000 tunnel2=1000000 ;;
*)
    echo "$usage" >&2
    exit 2
    ;;
esac
cat <<REPLAY
# The first $area level with seed $seed, standing still (fidelity/gen-area.sh $area $seed $frames).
seed $seed
frames $frames
highscore $money "MONEY"
highscore 8000000 "TIME"
highscore 7000000 "KILLS"
highscore 6000000 "SAVES"
highscore 5000000 "PLAYS"
highscore 4000000 "WINS"
highscore 3000000 "DEATHS"
highscore $tunnel1 "TUNNEL1"
highscore $tunnel2 "TUNNEL2"
highscore 0 "MINIGAMES"
20 hold X 2
60 hold X 2
REPLAY
case "$area" in
mines) printf '%s\n' "450 hold left 70" "535 hold up 5" ;;
jungle) printf '%s\n' "450 hold left 81" "538 hold up 100" "640 hold left 99" "640 hold Z 6" "750 hold up 5" ;;
ice) printf '%s\n' "450 hold left 81" "538 hold up 100" "640 hold left 76" "640 hold Z 6" "730 hold up 5" ;;
esac
