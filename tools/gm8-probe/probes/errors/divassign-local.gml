// a /= 0 on a local: the same check as a / 0?
var a, z;
z = 0;
a = 1;
a /= z;
out("divassign_local", a);
