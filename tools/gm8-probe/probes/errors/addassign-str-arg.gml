// argument0 += 1 on a string: OpenGMK makes op= on argumentN strict about types (an error). Is GM8?
// (divassign-str-arg2 found `argument0 /= 2` keeps the string without an error.)
argument0 = "abc";
argument0 += 1;
out("addassign_str_arg", argument0);
