// A string as an if condition (851-3798): GM8 stops ("Expression expected"?); OpenGMK reads it as false.
s = "abc";
t = 0;
if (s) t = 1;
out("if_str", t);
