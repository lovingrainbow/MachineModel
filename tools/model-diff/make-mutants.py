#!/usr/bin/env python3
"""由 fixtures/base_same_types 產生違規樣本到 fixtures/mut_*（兩邊用相同本地類型）。
每個樣本 = (名稱, 取代規則)；取代規則是 (舊字串, 新字串) 或附加到檔尾的文字。"""
import shutil, pathlib, sys
root = pathlib.Path(__file__).parent / "fixtures"
base = root / "base_same_types"
text = (base / "machine.yaml").read_text(encoding="utf-8")
M = {
 "unknown_tag_field": ("{ id: plc1.M_Spare, address: M130, type: bool }", "{ id: plc1.M_Spare, address: M130, type: bool, writable: true }"),
 "bad_address": ("address: M130", "address: ZZ9"),
 "dup_tag_id": ("{ id: plc1.M_Spare, address: M130, type: bool }", "{ id: plc1.M_Estop, address: M130, type: bool }"),
 "bad_type": ("address: M130, type: bool", "address: M130, type: string:0"),
 "string_array": ("address: M130, type: bool", "address: M130, type: string:8[3]"),
 "unknown_family": ("family: inovance_easy }", "family: no_such_plc }"),
 "schema_v1": ("schema_version: 2", "schema_version: 1"),
 "unknown_unit_type": ("type: cylinder_double", "type: nope"),
 "type_version": ("type: cylinder_double, type_version: 1", "type: cylinder_double, type_version: 9"),
 "unknown_bind_key": ("man_ext: plc1.M_Cyl1_ManExt", "man_xxx: plc1.M_Cyl1_ManExt"),
 "missing_required": ("lim_ext: plc1.X_Cyl1_Ext_LS, ", ""),
 "unknown_tag_ref": ("lim_ext: plc1.X_Cyl1_Ext_LS", "lim_ext: plc1.NOPE"),
 "man_on_input_addr": ("man_ext: plc1.M_Cyl1_ManExt", "man_ext: plc1.X_Cyl1_Ext_LS"),
 "output_bound_twice": ("valve_ret: plc1.Y_Cyl1_Ret", "valve_ret: plc1.Y_Cyl1_Ext"),
 "parent_cycle": ("bind: { cyl: cyl1 }", "bind: { cyl: st1 }"),
 "address_out_of_range": ("address: D100", "address: D99999999"),
 "polarity_non_bool": ("address: D100, type: int16", "address: D100, type: int16, polarity: NO"),
 "min_gt_max": ("address: D100, type: int16", "address: D100, type: int16, min: 10, max: 1"),
 "default_out_of_type": ("address: D100, type: int16", "address: D100, type: int16, default: 70000"),
 "scan_bad": ("address: D100, type: int16", "address: D100, type: int16, scan: warp"),
 "typo_neighbour": ("man_ext: plc1.M_Cyl1_ManExt", "man_exs: plc1.M_Cyl1_ManExt"),
 "variable_with_address": (None, "variables:\n  - { id: plc1.V_x, type: int16, address: D1 }\n"),
 "empty_plcs": ("plcs:\n    - { id: plc1, family: inovance_easy }", "plcs: []"),
}
for name, (old, new) in M.items():
    out = root / f"mut_{name}"
    if out.exists():
        shutil.rmtree(out)
    shutil.copytree(base, out)
    if old is None:
        t = text + new
    else:
        if old not in text:
            sys.exit(f"{name}: 找不到要取代的字串 {old!r}")
        t = text.replace(old, new, 1)
    (out / "machine.yaml").write_text(t, encoding="utf-8")
print(len(M), "個樣本")
