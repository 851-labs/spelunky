// A string as a do-until condition (851-3798).
s = "abc";
t = 0;
do { t += 1; if (t > 2) break; } until (s);
out("until_str", t);
