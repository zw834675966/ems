#!/usr/bin/env python3
"""
Send a TCP.md binary frame to a device/gateway (no third-party deps).

Frame:
| SOF(0xAA55) | VER(0x01) | FRAME_LEN(u16) | DEV_COUNT(u8) | DEV_BLOCK... | CRC16(u16) |
DEV_BLOCK:
| DEV_ID(u16) | DEV_TYPE(u8) | DEV_LEN(u16) | DEV_PAYLOAD |
DEV_PAYLOAD:
TLV repeated: | TAG(u8) | LEN(u8) | VALUE(N) |

Examples:
  # Send one device block with voltage/current/power
  python3 scripts/tcp_send_frame.py --host 127.0.0.1 --port 9000 \\
    --dev-id 1 --dev-type 1 \\
    --tlv 1:uint16:3802 --tlv 2:uint16:1984 --tlv 3:int32:1196
"""

from __future__ import annotations

import argparse
import socket
import struct
import sys
import time
from typing import List, Tuple


def crc16_modbus(data: bytes) -> int:
    crc = 0xFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc & 0xFFFF


def encode_value(value_type: str, value: str, endian: str) -> bytes:
    endian = endian.lower()
    if endian not in {"big", "little"}:
        raise ValueError("endian must be big|little")
    t = value_type.lower()
    if t in {"uint8", "u8"}:
        v = int(value, 0)
        if not (0 <= v <= 255):
            raise ValueError("uint8 range 0..255")
        return struct.pack("B", v)
    if t in {"uint16", "u16"}:
        v = int(value, 0)
        if not (0 <= v <= 0xFFFF):
            raise ValueError("uint16 range 0..65535")
        return struct.pack(">H" if endian == "big" else "<H", v)
    if t in {"uint32", "u32"}:
        v = int(value, 0)
        if not (0 <= v <= 0xFFFFFFFF):
            raise ValueError("uint32 range 0..2^32-1")
        return struct.pack(">I" if endian == "big" else "<I", v)
    if t in {"int32", "i32"}:
        v = int(value, 0)
        if not (-0x80000000 <= v <= 0x7FFFFFFF):
            raise ValueError("int32 range -2^31..2^31-1")
        return struct.pack(">i" if endian == "big" else "<i", v)
    raise ValueError("unsupported valueType (use uint8/uint16/uint32/int32)")


def build_frame_one_device(
    dev_id: int,
    dev_type: int,
    tlvs: List[Tuple[int, bytes]],
    ver: int = 1,
) -> bytes:
    dev_payload = bytearray()
    for tag, value in tlvs:
        if not (0 <= tag <= 255):
            raise ValueError("tag must be 0..255")
        if len(value) > 255:
            raise ValueError("TLV value too long (>255)")
        dev_payload.append(tag)
        dev_payload.append(len(value))
        dev_payload.extend(value)

    payload = bytearray()
    payload.append(ver & 0xFF)  # VER
    payload.extend(b"\x00\x00")  # FRAME_LEN placeholder
    payload.append(1)  # DEV_COUNT
    payload.extend(struct.pack(">H", dev_id))
    payload.append(dev_type & 0xFF)
    payload.extend(struct.pack(">H", len(dev_payload)))
    payload.extend(dev_payload)

    frame_len = len(payload)
    payload[1:3] = struct.pack(">H", frame_len)
    crc = crc16_modbus(bytes(payload))

    frame = bytearray(b"\xAA\x55")
    frame.extend(payload)
    frame.extend(struct.pack(">H", crc))
    return bytes(frame)


def main() -> int:
    ap = argparse.ArgumentParser(prog="tcp_send_frame.py")
    ap.add_argument("--host", required=True)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--dev-id", type=int, required=True)
    ap.add_argument("--dev-type", type=int, default=1)
    ap.add_argument("--endian", default="big", help="big|little for TLV VALUE encoding")
    ap.add_argument(
        "--tlv",
        action="append",
        default=[],
        help="format: <tag>:<type>:<value>  e.g. 1:uint16:3802",
    )
    ap.add_argument("--repeat", type=int, default=1, help="send N times")
    ap.add_argument("--interval-ms", type=int, default=1000)
    ap.add_argument("--timeout", type=float, default=2.0)
    args = ap.parse_args()

    tlvs: List[Tuple[int, bytes]] = []
    for item in args.tlv:
        try:
            tag_s, type_s, value_s = item.split(":", 2)
        except ValueError:
            raise SystemExit("invalid --tlv, expected tag:type:value")
        tag = int(tag_s, 0)
        value = encode_value(type_s, value_s, args.endian)
        tlvs.append((tag, value))
    if not tlvs:
        raise SystemExit("at least one --tlv is required")

    frame = build_frame_one_device(args.dev_id, args.dev_type, tlvs)
    for i in range(args.repeat):
        with socket.create_connection((args.host, args.port), timeout=args.timeout) as sock:
            sock.sendall(frame)
        if i + 1 < args.repeat:
            time.sleep(max(args.interval_ms, 0) / 1000.0)
    print(f"sent {args.repeat} frame(s), bytes={len(frame)} crc16=0x{crc16_modbus(frame[2:-2]):04X}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as e:
        print(f"error: {e}", file=sys.stderr)
        raise SystemExit(2)

