if (isRoom("rCredits1") and instance_exists(oCamel))
{
    x += 0.01;
}
else if (view_xview[0] != 0)
{
    xOff -= 0.01;
}
x = view_xview[0] + 208 + xOff;
