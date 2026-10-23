// Synthetic: every GM8 syntax quirk the parser accepts, whether or not Spelunky uses it.
var a b, c
globalvar debugMode;
a := $1F + #ff + .5 + 2.25
b = 'single "quoted"' + "double 'quoted'"
if a <> b then c = 1 else c = 2
if not a and b or c xor d { e = 1 }
while x > 0 do x -= 1
do begin y += 1; end until y >= 10
with oSnake do hp = 0
with (other) { other.x = x }
(obj).x = 3;;;
self.y = global.z
n = 7 div 2 + 7 mod 2 + 7 % 2
m = a = b
f = 1 | 2 & 3 ^ 4 << 1 >> 1
g = -a * ~b + !c
for (i = 0 i < 3 i += 1) arr[i, 0] = noone
for (j = 0; j < all; j += 1;) { continue; }
repeat (3) break
switch (n) { case 1: case 2: m *= 2; break; default: m /= 2 }
if (q) ; else exit
return f(g(1), "x", h())
/* a block comment
   over two lines */
