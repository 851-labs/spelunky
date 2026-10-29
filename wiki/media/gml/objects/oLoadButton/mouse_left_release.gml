if (sprite_index == sLoadButtonPressed and active)
{
    with oLoadLevel
    {
        global.firstCustomLevel = levelName;
        scrLoadLevel();
    }
}
sprite_index = sLoadButton;
