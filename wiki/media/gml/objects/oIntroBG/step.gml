if (isRoom("rCredits1") and instance_exists(oCamel))
{
    x += 0.02;
}
else if (view_xview[0] != 0)
{
    xOff -= 0.02;
}
x = view_xview[0] + xOff;
