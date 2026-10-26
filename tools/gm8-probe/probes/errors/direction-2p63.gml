// direction = d wraps as d - 360 * Floor(d / 360) (motion-setters, 851-3280), and Delphi's Floor is Trunc into
// an Int64 cut to a 32-bit Integer. Does a quotient from 2^63 up stop the game, as Trunc's FISTP traps for
// floor(2^63) (numbers)? At speed 0: with a speed, the components' snap traps first (fsin and fcos of an
// angle past 2^63 radians return it unchanged; errors/speedwrite-2p63).
var p;
p = 1; repeat (63) p = p * 2;
speed = 0;
direction = 360 * (p - 1024);
out("direction_360x(2p63-1024)", direction);
out("hspeed_360x(2p63-1024)", hspeed);
// That wraps to 360 * 2^63 (-1024 is -1024 in 32 bits), so a write of 360 * 2^63 would be skipped.
direction = 360 * p * 2;
out("direction_360x2p64", direction);
out("hspeed_360x2p64", hspeed);
