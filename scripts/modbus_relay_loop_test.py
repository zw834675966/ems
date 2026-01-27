#!/usr/bin/env python3
"""
Continuous Modbus TCP relay test (no third-party deps).

Goal:
- Write relay output to True (ON) on slave/unit 1 address 40001 (default).
- Read back output state and (optionally) read a feedback input; require True.
- Loop forever and exit non-zero on the first failure, or stop after N successes.

Address formats:
- Use `--out-ref/--fb-ref` with common Modbus reference numbers:
  - 00001.. -> coils (FC01 read / FC05 write)
  - 10001.. -> discrete inputs (FC02 read)
  - 30001.. -> input registers (FC04 read)
  - 40001.. -> holding registers (FC03 read / FC06 write)

Notes:
- Many devices use 0-based offsets on the wire; 40001 corresponds to addr=0.
"""

from __future__ import annotations

import argparse
import struct
import sys
import time
from dataclasses import dataclass

# Reuse the existing minimal Modbus TCP implementation in this repo.
try:
    from scripts import modbus_tcp_tool as mb
except Exception:
    # Allow running as `python3 scripts/modbus_relay_loop_test.py` from repo root.
    import os

    sys.path.insert(0, os.path.join(os.path.dirname(__file__)))
    import modbus_tcp_tool as mb  # type: ignore


@dataclass(frozen=True)
class Ref:
    read_fc: int
    write_fc: int | None
    addr0: int
    name: str


def parse_ref(s: str, *, for_write: bool) -> Ref:
    s = s.strip()
    if not s:
        raise ValueError("empty ref")
    if not s.isdigit():
        raise ValueError("ref must be digits like 40001/00001/10001")
    n = int(s, 10)
    if n <= 0:
        raise ValueError("ref must be > 0")

    first = s[0]
    if first == "0":
        # 00001 -> coil 0
        addr0 = n - 1
        return Ref(read_fc=1, write_fc=5 if for_write else None, addr0=addr0, name=f"coil {s}")
    if first == "1":
        addr0 = n - 10001
        return Ref(read_fc=2, write_fc=None, addr0=addr0, name=f"discrete input {s}")
    if first == "3":
        addr0 = n - 30001
        return Ref(read_fc=4, write_fc=None, addr0=addr0, name=f"input register {s}")
    if first == "4":
        addr0 = n - 40001
        return Ref(read_fc=3, write_fc=6 if for_write else None, addr0=addr0, name=f"holding register {s}")
    raise ValueError("ref must start with 0/1/3/4 (e.g. 40001)")


def read_bool(host: str, port: int, unit_id: int, timeout: float, r: Ref) -> bool:
    if r.addr0 < 0 or r.addr0 > 65535:
        raise ValueError(f"{r.name}: address out of range after conversion: {r.addr0}")
    pdu = struct.pack(">BHH", r.read_fc, r.addr0, 1)
    _, rx_pdu = mb.modbus_request(host, port, unit_id, pdu, timeout)
    if not rx_pdu:
        raise RuntimeError(f"{r.name}: empty response")
    if rx_pdu[0] == (r.read_fc | 0x80):
        raise RuntimeError(mb.decode_exception(r.read_fc, rx_pdu))
    if rx_pdu[0] != r.read_fc:
        raise RuntimeError(f"{r.name}: unexpected fc=0x{rx_pdu[0]:02X}")

    if r.read_fc in (1, 2):
        if len(rx_pdu) < 3:
            raise RuntimeError(f"{r.name}: truncated bits response")
        byte_count = rx_pdu[1]
        data = rx_pdu[2 : 2 + byte_count]
        if not data:
            return False
        return bool(data[0] & 0x01)

    # registers (FC03/FC04)
    if len(rx_pdu) < 4:
        raise RuntimeError(f"{r.name}: truncated register response")
    byte_count = rx_pdu[1]
    data = rx_pdu[2 : 2 + byte_count]
    if len(data) < 2:
        return False
    v = struct.unpack(">H", data[:2])[0]
    return v != 0


def write_true(host: str, port: int, unit_id: int, timeout: float, r: Ref) -> None:
    if r.write_fc is None:
        raise ValueError(f"{r.name}: not writable")
    if r.addr0 < 0 or r.addr0 > 65535:
        raise ValueError(f"{r.name}: address out of range after conversion: {r.addr0}")

    if r.write_fc == 5:
        val = 0xFF00
        pdu = struct.pack(">BHH", r.write_fc, r.addr0, val)
    elif r.write_fc == 6:
        pdu = struct.pack(">BHH", r.write_fc, r.addr0, 1)
    else:
        raise ValueError(f"{r.name}: unsupported write fc={r.write_fc}")

    _, rx_pdu = mb.modbus_request(host, port, unit_id, pdu, timeout)
    if not rx_pdu:
        raise RuntimeError(f"{r.name}: empty response on write")
    if rx_pdu[0] == (r.write_fc | 0x80):
        raise RuntimeError(mb.decode_exception(r.write_fc, rx_pdu))


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="modbus_relay_loop_test.py",
        description="Continuously write relay output and verify output/feedback are True.",
    )
    p.add_argument("--host", required=True, help="gateway ip, e.g. 192.168.0.80")
    p.add_argument("--port", type=int, default=502)
    p.add_argument("--unit-id", type=int, default=1)
    p.add_argument("--timeout", type=float, default=1.0, help="socket timeout seconds")

    p.add_argument("--out-ref", default="40001", help="output ref, default 40001")
    p.add_argument(
        "--fb-ref",
        default=None,
        help="optional feedback ref to also verify True (e.g. 10001/40002)",
    )

    p.add_argument("--poll-ms", type=int, default=100, help="poll interval while waiting")
    p.add_argument("--settle-timeout", type=float, default=2.0, help="seconds to wait for True")
    p.add_argument(
        "--required-successes",
        type=int,
        default=0,
        help="exit 0 after N consecutive successes (0 means run forever until failure)",
    )
    p.add_argument("--verbose", action="store_true")
    return p


def main() -> int:
    args = build_parser().parse_args()
    if not (0 <= args.unit_id <= 247):
        print("unit-id must be 0..247", file=sys.stderr)
        return 2
    if args.poll_ms <= 0:
        print("poll-ms must be > 0", file=sys.stderr)
        return 2
    if args.settle_timeout <= 0:
        print("settle-timeout must be > 0", file=sys.stderr)
        return 2
    if args.required_successes < 0:
        print("required-successes must be >= 0", file=sys.stderr)
        return 2

    out = parse_ref(str(args.out_ref), for_write=True)
    fb = parse_ref(str(args.fb_ref), for_write=False) if args.fb_ref else None

    successes = 0
    iteration = 0
    while True:
        iteration += 1
        start = time.time()
        try:
            write_true(args.host, args.port, args.unit_id, args.timeout, out)

            deadline = time.time() + args.settle_timeout
            out_ok = False
            fb_ok = False if fb else True
            last_out = None
            last_fb = None
            while time.time() <= deadline:
                last_out = read_bool(args.host, args.port, args.unit_id, args.timeout, out)
                out_ok = last_out is True
                if fb:
                    last_fb = read_bool(args.host, args.port, args.unit_id, args.timeout, fb)
                    fb_ok = last_fb is True
                if out_ok and fb_ok:
                    break
                time.sleep(args.poll_ms / 1000.0)

            if not (out_ok and fb_ok):
                msg = f"FAIL iter={iteration} out={last_out}"
                if fb:
                    msg += f" fb={last_fb}"
                msg += f" waited={time.time()-start:.3f}s"
                print(msg, file=sys.stderr)
                return 1

            successes += 1
            if args.verbose:
                msg = f"ok iter={iteration} out=True"
                if fb:
                    msg += " fb=True"
                msg += f" elapsed={time.time()-start:.3f}s successes={successes}"
                print(msg)

            if args.required_successes and successes >= args.required_successes:
                return 0
        except KeyboardInterrupt:
            print("stopped by user", file=sys.stderr)
            return 130
        except Exception as e:
            print(f"FAIL iter={iteration} error={e}", file=sys.stderr)
            return 1


if __name__ == "__main__":
    raise SystemExit(main())

