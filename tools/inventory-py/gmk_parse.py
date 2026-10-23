#!/usr/bin/env python3
"""Throwaway GameMaker 8.0 .gmk parser (data only, nothing is executed).

Format reference: LateralGM org/lateralgm/file/GmFileReader.java + GmStreamDecoder.java.
Usage: python3 -I gmk_parse.py <file.gmk> <outdir>
"""
import json
import os
import re
import struct
import sys
import zlib

ENC = "cp1252"  # GM8.0 uses the ANSI code page

EVENT_NAMES = {0: "create", 1: "destroy", 2: "alarm", 3: "step", 4: "collision",
               5: "keyboard", 6: "mouse", 7: "other", 8: "draw", 9: "keypress",
               10: "keyrelease", 11: "trigger"}
STEP_SUB = {0: "step", 1: "begin_step", 2: "end_step"}
OTHER_SUB = {0: "outside", 1: "boundary", 2: "game_start", 3: "game_end", 4: "room_start",
             5: "room_end", 6: "no_more_lives", 7: "animation_end", 8: "end_of_path",
             9: "no_more_health", 30: "close_button"}
for _i in range(16):
    OTHER_SUB[10 + _i] = "user%d" % _i
MOUSE_SUB = {0: "left_button", 1: "right_button", 2: "middle_button", 3: "no_button",
             4: "left_press", 5: "right_press", 6: "middle_press", 7: "left_release",
             8: "right_release", 9: "middle_release", 10: "mouse_enter", 11: "mouse_leave",
             60: "wheel_up", 61: "wheel_down", 50: "global_left_button",
             51: "global_right_button", 52: "global_middle_button", 53: "global_left_press",
             54: "global_right_press", 55: "global_middle_press", 56: "global_left_release",
             57: "global_right_release", 58: "global_middle_release"}
ACTION_KIND = {0: "normal", 1: "begin_group", 2: "end_group", 3: "else", 4: "exit",
               5: "repeat", 6: "variable", 7: "code"}
EXEC_TYPE = {0: "none", 1: "function", 2: "code"}
ARG_KIND = {0: "expression", 1: "string", 2: "both", 3: "boolean", 4: "menu", 5: "sprite",
            6: "sound", 7: "background", 8: "path", 9: "script", 10: "object", 11: "room",
            12: "font", 13: "color", 14: "timeline", 15: "font_string"}
SOUND_KIND = {0: "normal", 1: "background", 2: "3d", 3: "multimedia"}
BB_MODE = {0: "automatic", 1: "full_image", 2: "manual"}
MASK_SHAPE = {0: "precise", 1: "rectangle", 2: "disk", 3: "diamond", 4: "polygon"}
RES_KIND = {1: "object", 2: "sprite", 3: "sound", 4: "room", 6: "background", 7: "script",
            8: "path", 9: "font", 10: "game_information", 11: "game_settings",
            12: "timeline", 13: "extension_packages"}
VK = {8: "backspace", 9: "tab", 13: "enter", 16: "shift", 17: "control", 18: "alt",
      19: "pause", 27: "escape", 32: "space", 33: "pageup", 34: "pagedown", 35: "end",
      36: "home", 37: "left", 38: "up", 39: "right", 40: "down", 45: "insert", 46: "delete",
      0: "nokey", 1: "anykey"}


class FormatError(Exception):
    pass


class R:
    def __init__(self, data, name="file", base=0):
        self.d = data
        self.p = 0
        self.name = name
        self.base = base

    def need(self, n):
        if self.p + n > len(self.d):
            raise FormatError("%s: read past end at %d (+%d, len %d)" % (self.name, self.p, n, len(self.d)))

    def i32(self):
        self.need(4)
        v = struct.unpack_from("<i", self.d, self.p)[0]
        self.p += 4
        return v

    def u32(self):
        self.need(4)
        v = struct.unpack_from("<I", self.d, self.p)[0]
        self.p += 4
        return v

    def u16(self):
        self.need(2)
        v = struct.unpack_from("<H", self.d, self.p)[0]
        self.p += 2
        return v

    def u8(self):
        self.need(1)
        v = self.d[self.p]
        self.p += 1
        return v

    def f64(self):
        self.need(8)
        v = struct.unpack_from("<d", self.d, self.p)[0]
        self.p += 8
        return v

    def bool(self):
        v = self.i32()
        if v not in (0, 1):
            raise FormatError("%s: invalid boolean %d at %d" % (self.name, v, self.p - 4))
        return v == 1

    def raw(self, n):
        if n < 0:
            raise FormatError("%s: negative length %d at %d" % (self.name, n, self.p))
        self.need(n)
        v = self.d[self.p:self.p + n]
        self.p += n
        return v

    def str(self):
        n = self.i32()
        return self.raw(n).decode(ENC, errors="replace")

    def blob(self):
        return self.raw(self.i32())

    def inflate(self, name):
        n = self.i32()
        comp = self.raw(n)
        try:
            data = zlib.decompress(comp)
        except zlib.error as e:
            raise FormatError("%s: zlib error: %s" % (name, e))
        return R(data, name)

    def expect(self, allowed, what):
        v = self.i32()
        if v not in allowed:
            raise FormatError("%s: unexpected %s version %d at %d" % (self.name, what, v, self.p - 4))
        return v

    def done(self, warnings):
        if self.p != len(self.d):
            warnings.append("%s: %d trailing bytes not consumed" % (self.name, len(self.d) - self.p))


def ver_check(r, allowed, what):
    return r.expect(allowed, what)


def read_settings(r, W):
    ver = r.expect([800], "settings")
    s = R.inflate(r, "settings")
    g = {}
    g["start_fullscreen"] = s.bool()
    g["interpolate_pixels"] = s.bool()
    g["dont_draw_border"] = s.bool()
    g["display_cursor"] = s.bool()
    g["scaling"] = s.i32()  # -1 keep aspect, 0 full scale, >0 fixed %
    g["allow_window_resize"] = s.bool()
    g["always_on_top"] = s.bool()
    g["color_outside_room_bgr"] = s.u32()
    g["set_resolution"] = s.bool()
    g["color_depth"] = s.i32()
    g["resolution"] = s.i32()
    g["frequency"] = s.i32()
    g["dont_show_buttons"] = s.bool()
    sync = s.u32()
    g["use_vsync"] = bool(sync & 1)
    g["disable_screensavers"] = s.bool()
    g["f4_fullscreen"] = s.bool()
    g["f1_game_info"] = s.bool()
    g["esc_ends_game"] = s.bool()
    g["f5_save_f6_load"] = s.bool()
    g["f9_screenshot"] = s.bool()
    g["close_as_esc"] = s.bool()
    g["priority"] = s.i32()
    g["freeze_on_lose_focus"] = s.bool()
    g["load_bar_mode"] = s.i32()
    if g["load_bar_mode"] == 2:
        if s.bool():
            g["back_load_bar_bytes"] = len(s.blob())
        if s.bool():
            g["front_load_bar_bytes"] = len(s.blob())
    g["show_custom_load_image"] = s.bool()
    if g["show_custom_load_image"]:
        if s.bool():
            g["custom_load_image_bytes"] = len(s.blob())
    g["load_image_partially_transparent"] = s.bool()
    g["load_image_alpha"] = s.i32()
    g["scale_progress_bar"] = s.bool()
    g["icon_bytes"] = len(s.blob())
    g["display_errors"] = s.bool()
    g["write_errors_to_log"] = s.bool()
    g["abort_on_error"] = s.bool()
    errs = s.i32()
    g["treat_uninitialized_as_0"] = bool(errs & 1)
    g["error_on_args_raw"] = errs
    g["author"] = s.str()
    g["version"] = s.str()
    g["last_changed"] = s.f64()
    g["information"] = s.str()
    g["version_major"], g["version_minor"], g["version_release"], g["version_build"] = (
        s.i32(), s.i32(), s.i32(), s.i32())
    g["company"] = s.str()
    g["product"] = s.str()
    g["copyright"] = s.str()
    g["description"] = s.str()
    s.f64()  # last changed
    s.done(W)
    return g


def read_triggers(r, W):
    r.expect([800], "triggers")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("trigger[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        s.expect([800], "trigger")
        out.append({"index": i, "name": s.str(), "condition": s.str(), "check_moment": s.i32(),
                    "constant_name": s.str()})
        s.done(W)
    r.f64()
    return out


def read_constants(r, W):
    r.expect([800], "constants")
    n = r.i32()
    out = []
    for i in range(n):
        out.append({"name": r.str(), "value": r.str()})
    r.f64()
    return out


def read_sounds(r, W):
    r.expect([800], "sounds section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("sound[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([800], "sound")
        d["kind_id"] = s.i32()
        d["kind"] = SOUND_KIND.get(d["kind_id"], "?")
        d["file_type"] = s.str()
        d["file_name"] = s.str()
        if s.bool():
            data = s.blob()
            d["data_bytes"] = len(data)
            d["data_magic"] = data[:4].decode("latin1", "replace")
        else:
            d["data_bytes"] = 0
        fx = s.i32()
        d["effects"] = [nm for b, nm in enumerate(["chorus", "echo", "flanger", "gargle", "reverb"]) if fx & (1 << b)]
        d["volume"] = s.f64()
        d["pan"] = s.f64()
        d["preload"] = s.bool()
        s.done(W)
        out.append(d)
    return out


def read_sprites(r, W):
    r.expect([800], "sprites section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("sprite[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([800], "sprite")
        d["origin_x"] = s.i32()
        d["origin_y"] = s.i32()
        nsub = s.i32()
        frames = []
        ok = True
        for j in range(nsub):
            s.expect([800], "subimage")
            w, h = s.i32(), s.i32()
            if w != 0 and h != 0:
                ln = s.i32()
                if ln != w * h * 4:
                    ok = False
                s.raw(ln)
            frames.append((w, h))
        d["frames"] = nsub
        sizes = sorted(set(frames))
        d["width"] = frames[0][0] if frames else 0
        d["height"] = frames[0][1] if frames else 0
        if len(sizes) > 1:
            d["frame_sizes"] = sizes
        d["pixels_bgra_decodable"] = ok
        d["mask_shape"] = MASK_SHAPE.get(s.i32(), "?")
        d["alpha_tolerance"] = s.i32()
        d["separate_masks"] = s.bool()
        d["bbox_mode"] = BB_MODE.get(s.i32(), "?")
        d["bbox_left"], d["bbox_right"], d["bbox_bottom"], d["bbox_top"] = s.i32(), s.i32(), s.i32(), s.i32()
        s.done(W)
        out.append(d)
    return out


def read_backgrounds(r, W):
    r.expect([800], "backgrounds section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("background[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([710], "background")
        d["use_as_tileset"] = s.bool()
        d["tile_width"], d["tile_height"], d["h_offset"], d["v_offset"], d["h_sep"], d["v_sep"] = [s.i32() for _ in range(6)]
        s.expect([800], "background image")
        w, h = s.i32(), s.i32()
        d["width"], d["height"] = w, h
        ok = True
        if w and h:
            ln = s.i32()
            ok = ln == w * h * 4
            s.raw(ln)
        d["pixels_bgra_decodable"] = ok
        s.done(W)
        out.append(d)
    return out


def read_paths(r, W):
    r.expect([800], "paths section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("path[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([530], "path")
        d["smooth"] = s.bool()
        d["closed"] = s.bool()
        d["precision"] = s.i32()
        d["background_room"] = s.i32()
        d["snap_x"], d["snap_y"] = s.i32(), s.i32()
        npts = s.i32()
        d["points"] = [{"x": s.f64(), "y": s.f64(), "speed": s.f64()} for _ in range(npts)]
        s.done(W)
        out.append(d)
    return out


def read_scripts(r, W):
    r.expect([800], "scripts section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("script[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([800], "script")
        d["code"] = s.str()
        s.done(W)
        out.append(d)
    return out


def read_fonts(r, W):
    r.expect([800], "fonts section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("font[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([800], "font")
        d["font_name"] = s.str()
        d["size"] = s.i32()
        d["bold"] = s.bool()
        d["italic"] = s.bool()
        d["range_min"] = s.u16()
        d["charset"] = s.u8()
        d["antialias_raw"] = s.u8()  # 0 in GM8.0 files
        d["range_max"] = s.i32()
        s.done(W)
        out.append(d)
    return out


def read_actions(s, ctx):
    s.expect([400], "action list (%s)" % ctx)
    n = s.i32()
    acts = []
    for k in range(n):
        a = {}
        s.expect([440], "action (%s)" % ctx)
        a["lib"] = s.i32()
        a["id"] = s.i32()
        kind = s.i32()
        a["kind"] = ACTION_KIND.get(kind, kind)
        a["may_be_relative"] = s.bool()
        a["is_question"] = s.bool()
        a["has_applies_to"] = s.bool()
        et = s.i32()
        a["exec_type"] = EXEC_TYPE.get(et, et)
        a["function"] = s.str()
        a["exec_code"] = s.str()
        a["arg_count"] = s.i32()
        nk = s.i32()
        kinds = [s.i32() for _ in range(nk)]
        a["arg_kinds"] = [ARG_KIND.get(x, x) for x in kinds[:a["arg_count"]]]
        a["applies_to"] = s.i32()  # -1 self, -2 other, else object id
        a["relative"] = s.bool()
        na = s.i32()
        args = [s.str() for _ in range(na)]
        a["args"] = args[:a["arg_count"]]
        a["not"] = s.bool()
        acts.append(a)
    return acts


def read_timelines(r, W):
    r.expect([800], "timelines section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("timeline[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([500], "timeline")
        nm = s.i32()
        d["moments"] = []
        for j in range(nm):
            step = s.i32()
            d["moments"].append({"step": step, "actions": read_actions(s, "timeline %d" % i)})
        s.done(W)
        out.append(d)
    return out


def read_objects(r, W):
    r.expect([800], "objects section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("object[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([430], "object")
        d["sprite"] = s.i32()
        d["solid"] = s.bool()
        d["visible"] = s.bool()
        d["depth"] = s.i32()
        d["persistent"] = s.bool()
        d["parent"] = s.i32()
        d["mask"] = s.i32()
        nev = s.i32() + 1
        events = []
        for j in range(nev):
            while True:
                first = s.i32()
                if first == -1:
                    break
                acts = read_actions(s, "object %s event %d/%d" % (d["name"], j, first))
                events.append({"type": j, "sub": first, "actions": acts})
        d["events"] = events
        s.done(W)
        out.append(d)
    return out


def read_rooms(r, W):
    r.expect([800], "rooms section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("room[%d]" % i)
        if not s.bool():
            out.append(None)
            s.done(W)
            continue
        d = {"id": i, "name": s.str()}
        s.f64()
        s.expect([541], "room")
        d["caption"] = s.str()
        d["width"], d["height"], d["snap_y"], d["snap_x"] = s.i32(), s.i32(), s.i32(), s.i32()
        d["isometric"] = s.bool()
        d["speed"] = s.i32()
        d["persistent"] = s.bool()
        d["background_color_bgr"] = s.u32()
        d["draw_background_color"] = s.bool()
        d["creation_code"] = s.str()
        nb = s.i32()
        d["backgrounds"] = []
        for j in range(nb):
            b = {"index": j, "visible": s.bool(), "foreground": s.bool(), "background": s.i32(),
                 "x": s.i32(), "y": s.i32(), "tile_h": s.bool(), "tile_v": s.bool(),
                 "hspeed": s.i32(), "vspeed": s.i32(), "stretch": s.bool()}
            d["backgrounds"].append(b)
        d["views_enabled"] = s.bool()
        nv = s.i32()
        d["views"] = []
        for j in range(nv):
            v = {"index": j, "visible": s.bool()}
            for k in ["view_x", "view_y", "view_w", "view_h", "port_x", "port_y", "port_w",
                      "port_h", "hborder", "vborder", "hspeed", "vspeed", "follow_object"]:
                v[k] = s.i32()
            d["views"].append(v)
        ni = s.i32()
        d["instances"] = []
        for j in range(ni):
            x, y, obj, iid = s.i32(), s.i32(), s.i32(), s.i32()
            cc = s.str()
            locked = s.bool()
            d["instances"].append({"x": x, "y": y, "object": obj, "id": iid, "creation_code": cc,
                                   "locked": locked})
        nt = s.i32()
        tiles = []
        for j in range(nt):
            t = {"x": s.i32(), "y": s.i32(), "background": s.i32(), "bg_x": s.i32(),
                 "bg_y": s.i32(), "w": s.i32(), "h": s.i32(), "depth": s.i32(), "id": s.i32(),
                 "locked": s.bool()}
            tiles.append(t)
        d["tiles"] = tiles
        d["editor"] = {"remember_window_size": s.bool(), "width": s.i32(), "height": s.i32(),
                       "show_grid": s.bool(), "show_objects": s.bool(), "show_tiles": s.bool(),
                       "show_backgrounds": s.bool(), "show_foregrounds": s.bool(),
                       "show_views": s.bool(), "delete_underlying_objects": s.bool(),
                       "delete_underlying_tiles": s.bool(), "tab": s.i32(), "scroll_x": s.i32(),
                       "scroll_y": s.i32()}
        s.done(W)
        out.append(d)
    return out


def read_includes(r, W):
    r.expect([800], "include files section")
    n = r.i32()
    out = []
    for i in range(n):
        s = r.inflate("include[%d]" % i)
        s.f64()
        s.expect([800], "include")
        d = {"file_name": s.str(), "file_path": s.str(), "original": s.bool(), "size": s.i32()}
        d["stored"] = s.bool()
        if d["stored"]:
            d["stored_bytes"] = len(s.blob())
        d["export"] = s.i32()
        d["export_folder"] = s.str()
        d["overwrite"] = s.bool()
        d["free_memory"] = s.bool()
        d["remove_at_end"] = s.bool()
        s.done(W)
        out.append(d)
    return out


def read_packages(r, W):
    r.expect([700], "extension packages")
    return [r.str() for _ in range(r.i32())]


def read_game_info(r, W):
    r.expect([800], "game information")
    s = r.inflate("game information")
    d = {"bg_color": s.i32(), "separate_window": s.bool(), "caption": s.str(),
         "left": s.i32(), "top": s.i32(), "width": s.i32(), "height": s.i32(),
         "show_border": s.bool(), "allow_resize": s.bool(), "stay_on_top": s.bool(),
         "pause_game": s.bool()}
    s.f64()
    d["text_rtf"] = s.str()
    s.done(W)
    return d


def read_tree(r, W):
    def node():
        status = r.i32()
        kind = r.i32()
        idx = r.i32()
        name = r.str()
        cnt = r.i32()
        n = {"status": {1: "primary", 2: "group", 3: "secondary"}.get(status, status),
             "kind": RES_KIND.get(kind, kind), "index": idx, "name": name}
        if cnt:
            n["children"] = [node() for _ in range(cnt)]
        return n
    return [node() for _ in range(12)]


def parse(path):
    data = open(path, "rb").read()
    r = R(data, os.path.basename(path))
    W = []
    G = {}
    if r.i32() != 1234321:
        raise FormatError("bad magic")
    G["format_version"] = r.expect([800], "gmk")
    G["game_id"] = r.i32()
    G["guid"] = r.raw(16).hex()
    stages = [("settings", read_settings), ("triggers", read_triggers),
              ("constants", read_constants), ("sounds", read_sounds),
              ("sprites", read_sprites), ("backgrounds", read_backgrounds),
              ("paths", read_paths), ("scripts", read_scripts), ("fonts", read_fonts),
              ("timelines", read_timelines), ("objects", read_objects), ("rooms", read_rooms)]
    for name, fn in stages:
        start = r.p
        G[name] = fn(r, W)
        G.setdefault("_offsets", {})[name] = [start, r.p]
    G["last_instance_id"] = r.i32()
    G["last_tile_id"] = r.i32()
    G["includes"] = read_includes(r, W)
    G["extension_packages"] = read_packages(r, W)
    G["game_information"] = read_game_info(r, W)
    r.expect([500], "library creation code")
    G["library_creation_code"] = [r.str() for _ in range(r.i32())]
    r.expect([700], "room order")
    G["room_order"] = [r.i32() for _ in range(r.i32())]
    G["resource_tree"] = read_tree(r, W)
    if r.p != len(data):
        W.append("top level: %d trailing bytes after resource tree" % (len(data) - r.p))
    G["_warnings"] = W
    G["_file_bytes"] = len(data)
    G["_consumed_bytes"] = r.p
    return G


if __name__ == "__main__":
    import pickle
    G = parse(sys.argv[1])
    os.makedirs(os.path.dirname(os.path.abspath(sys.argv[2])), exist_ok=True)
    with open(sys.argv[2], "wb") as f:
        pickle.dump(G, f)
    print("ok; warnings:", G["_warnings"])
    print("consumed", G["_consumed_bytes"], "of", G["_file_bytes"])
    for k in ["sounds", "sprites", "backgrounds", "paths", "scripts", "fonts", "timelines", "objects", "rooms"]:
        L = G[k]
        print(k, "slots", len(L), "defined", sum(1 for x in L if x))
    print("triggers", len([t for t in G["triggers"] if t]), "constants", len(G["constants"]),
          "includes", len(G["includes"]), "packages", G["extension_packages"])
