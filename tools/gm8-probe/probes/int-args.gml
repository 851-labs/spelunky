// Integer arguments and repeat counts in the Int64 range but past 32 bits: wrapped to 32 bits after Delphi's
// Round? Nothing here should error (past 2^63: the errors/ probes).
var p31, p32, p62, p63, n;
p31 = 2147483648;
p32 = 4294967296;
p62 = p31 * p31;
p63 = p62 * 2;
out("ia_chr_2p32+65", ord(chr(p32 + 65)));
out("ia_chr_65.5", ord(chr(65.5)));
out("ia_chr_66.5", ord(chr(66.5)));
out("ia_chr_-2p63", ord(chr(-p63)));
out("ia_chr_2p63-1024", ord(chr(p63 - 1024)));
out("ia_charat_2p32+2", string_char_at("abcdef", p32 + 2));
out("ia_charat_2p31+2", string_char_at("abcdef", p31 + 2));
out("ia_charat_-2p32+3", string_char_at("abcdef", -p32 + 3));
out("ia_color_2p32+1", make_color_rgb(p32 + 1, 0, 0));
out("ia_color_2p62+1024", make_color_rgb(p62 + 1024, 0, 0));
n = 0; repeat (p32 + 2) { n += 1; if (n >= 5) break; } out("ia_repeat_2p32+2", n);
n = 0; repeat (p31) { n += 1; if (n >= 5) break; } out("ia_repeat_2p31", n);
n = 0; repeat (p62) { n += 1; if (n >= 5) break; } out("ia_repeat_2p62", n);
n = 0; repeat (-p32 + 2) { n += 1; if (n >= 5) break; } out("ia_repeat_-2p32+2", n);
n = 0; repeat (-3) { n += 1; if (n >= 5) break; } out("ia_repeat_-3", n);
n = 0; repeat (p32 - 3) { n += 1; if (n >= 5) break; } out("ia_repeat_2p32-3", n);
n = 0; repeat (-p31) { n += 1; if (n >= 5) break; } out("ia_repeat_-2p31", n);
alarm[0] = p32 + 9;
out("ia_alarm_2p32+9", alarm[0]);
alarm[0] = -p63;
out("ia_alarm_-2p63", alarm[0]);
