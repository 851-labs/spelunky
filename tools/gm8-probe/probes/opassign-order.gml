// How often an op= evaluates its base and index (851-3798): `base.v[i] op= x` reads and writes through one
// evaluation of each in GM8; OpenGMK (`make_modify_instruction`) evaluates both twice. SS_FreeSound logs "b" and
// SS_PlaySound its argument, each returning its argument, so the lines are the calls in order.
var la;
global.probe_log = "";
pv = 10;
(SS_FreeSound(id)).pv += SS_PlaySound(2);
out("field_base", global.probe_log);
out("field_value", pv);
global.probe_log = "";
pa[1] = 10;
(SS_FreeSound(id)).pa[SS_PlaySound(1)] += SS_PlaySound(3);
out("field_base_index", global.probe_log);
out("field_base_index_value", pa[1]);
global.probe_log = "";
la[1] = 10;
la[SS_PlaySound(1)] += SS_PlaySound(2);
out("local_index", global.probe_log);
out("local_index_value", la[1]);
global.probe_log = "";
pa[1] = 10;
pa[SS_PlaySound(1)] *= SS_PlaySound(2);
out("field_index", global.probe_log);
out("field_index_value", pa[1]);
// script SS_FreeSound:
global.probe_log += "b";
return argument0;
// script SS_PlaySound:
global.probe_log += string(argument0);
return argument0;
