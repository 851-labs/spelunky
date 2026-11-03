// A string as a while condition (851-3798).
s = "abc";
t = 0;
while (s) { t += 1; if (t > 2) break; }
out("while_str", t);
