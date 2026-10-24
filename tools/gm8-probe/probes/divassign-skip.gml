// Does `v /= 0` skip the assignment, or write the old value back through the setter? Writing speed or
// direction back recomputes hspeed and vspeed (in the last bits, for these values); writing obj.x back
// copies the first instance's x to every instance.
var z, a, b, arr;
z = 0;
hspeed = 0.3;
vspeed = 0.7;
out("ds_h0", hspeed);
out("ds_v0", vspeed);
speed /= z;
out("ds_speed_h", hspeed);
out("ds_speed_v", vspeed);
direction /= z;
out("ds_dir_h", hspeed);
out("ds_dir_v", vspeed);
// The same writes with a divisor of 1, for comparison.
speed /= 1;
out("ds_speed1_h", hspeed);
out("ds_speed1_v", vspeed);
hspeed = 0.3;
vspeed = 0.7;
direction /= 1;
out("ds_dir1_h", hspeed);
out("ds_dir1_v", vspeed);
a = instance_create(10, 0, oBasket);
b = instance_create(20, 0, oBasket);
oBasket.x /= z;
out("ds_obj_a_x", a.x);
out("ds_obj_b_x", b.x);
oBasket.x /= 1;
out("ds_obj1_a_x", a.x);
out("ds_obj1_b_x", b.x);
arr[2] = 5;
arr[2] /= z;
out("ds_arr", arr[2]);
