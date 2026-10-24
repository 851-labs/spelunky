// argument0 /= 0 on a string: argumentN is strict about types in op=. Is the zero divisor checked first
// (no error), or the types (an error)? divassign-str-arg2 is the same with a divisor of 2.
var z;
z = 0;
argument0 = "abc";
argument0 /= z;
out("divassign_str_arg", argument0);
