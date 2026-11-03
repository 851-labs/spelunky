// A string as a for condition (851-3798).
s = "abc";
for (t = 0; s; t += 1) { if (t > 2) break; }
out("for_str", t);
