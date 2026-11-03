// `var` takes effect from its statement on (851-3798): a name used before it is still the instance's.
vv = 1;
var vv;
vv = 2;
out("self_vv", self.vv);
out("local_vv", vv);
ww = 5;
out("ww_before", ww);
var ww;
ww = 6;
out("self_ww", self.ww);
