// x /= 0 on a built-in variable (op= on built-ins is strict about types: is it about zero too?)
var z;
z = 0;
x = 5;
x /= z;
out("divassign_builtin", x);
