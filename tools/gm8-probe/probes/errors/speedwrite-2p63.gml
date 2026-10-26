// The other way round from speed-2p63: a speed write recomputes the components, each snapped through Delphi's
// Round into an Int64 (851-3281). Does a component from 2^63 up stop the game? (The port and the oracle don't
// check this snap yet: docs/quirks.md §7.)
var p;
p = 1; repeat (63) p = p * 2;
speed = p - 1024;
out("hspeed_speed_2p63-1024", hspeed);
speed = p;
out("hspeed_speed_2p63", hspeed);
