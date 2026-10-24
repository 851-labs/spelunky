// foo /= 0 on an instance variable (fields ignore type errors in op=; do they ignore this?)
var z;
z = 0;
foo = 1;
foo /= z;
out("divassign_field", foo);
