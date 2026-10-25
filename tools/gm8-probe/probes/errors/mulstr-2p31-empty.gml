// The empty string 2^31 times (mulstr-2p31 stops on "a"): if GM8 runs 2^31 appends there, this is "" with no error.
var p31;
p31 = 2147483648;
out("mulstr_2p31_empty", p31 * "");
