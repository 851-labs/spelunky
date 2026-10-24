var p;
p = 1; repeat (1023) p = p * 2; // 2^1023, exact
foo = p;
foo *= 2;
out("mulassign_overflow_field", foo);
