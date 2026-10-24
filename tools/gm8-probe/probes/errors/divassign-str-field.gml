// fs /= 0 and fs /= 2 on an instance variable holding a string (fields ignore type errors in op=)
var z;
z = 0;
fs = "abc";
fs /= z;
out("divassign_str_field_0", fs);
fs /= 2;
out("divassign_str_field_2", fs);
