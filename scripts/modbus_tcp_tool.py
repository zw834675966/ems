#!/usr/bin/env python3
"""
Minimal Modbus TCP test tool (no third-party deps).

Supports:
- FC01 Read Coils
- FC02 Read Discrete Inputs
- FC03 Read Holding Registers
- FC04 Read Input Registers
- FC05 Write Single Coil
- FC06 Write Single Register

Use this to validate "can talk to device" with the same address conventions
as `docs/protocols/Modbus TCP.md` (0-based offsets).
"""

from __future__ import annotations

import argparse
import socket
import struct
import sys
import time
from typing import Iterable, Tuple


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


def pack_mbap(transaction_id: int, unit_id: int, pdu: bytes) -> bytes:
    protocol_id = 0
    length = 1 + len(pdu)  # unit_id + pdu
    return struct.pack(">HHHB", transaction_id, protocol_id, length, unit_id) + pdu


def recv_exact(sock: socket.socket, n: int) -> bytes:
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise ConnectionError("socket closed")
        buf += chunk
    return buf


def modbus_request(
    host: str,
    port: int,
    unit_id: int,
    pdu: bytes,
    timeout: float,
) -> Tuple[int, bytes]:
    transaction_id = int(time.time() * 1000) & 0xFFFF
    adu = pack_mbap(transaction_id, unit_id, pdu)
    with socket.create_connection((host, port), timeout=timeout) as sock:
        sock.settimeout(timeout)
        sock.sendall(adu)

        # MBAP header: tid(2) pid(2) len(2) uid(1)
        mbap = recv_exact(sock, 7)
        rx_tid, rx_pid, rx_len, rx_uid = struct.unpack(">HHHB", mbap)
        if rx_pid != 0:
            raise ValueError(f"unexpected protocol id: {rx_pid}")
        if rx_uid != unit_id:
            raise ValueError(f"unexpected unit id: {rx_uid}")
        if rx_tid != transaction_id:
            # Some devices may not echo tid; warn but continue.
            pass
        pdu_len = rx_len - 1
        rx_pdu = recv_exact(sock, pdu_len)
        return rx_uid, rx_pdu


def decode_exception(fc: int, rx_pdu: bytes) -> str:
    if len(rx_pdu) < 2:
        return "exception: truncated"
    exc = rx_pdu[1]
    return f"exception fc=0x{fc:02X} code=0x{exc:02X}"


def parse_word_order(order: str) -> str:
    order = order.upper()
    if order not in {"ABCD", "CDAB", "BADC", "DCBA"}:
        raise ValueError("wordOrder must be one of ABCD/CDAB/BADC/DCBA")
    return order


def apply_word_order_32(b: bytes, order: str) -> bytes:
    # b is 4 bytes in ABCD order
    if order == "ABCD":
        return b
    if order == "CDAB":
        return b[2:4] + b[0:2]
    if order == "BADC":
        return bytes([b[1], b[0], b[3], b[2]])
    if order == "DCBA":
        return bytes([b[3], b[2], b[1], b[0]])
    raise ValueError("invalid wordOrder")


def decode_registers(
    regs: Iterable[int],
    data_type: str,
    endian: str,
    word_order: str | None,
) -> str:
    regs = list(regs)
    endian = endian.lower()
    data_type = data_type.lower()
    if endian not in {"big", "little"}:
        raise ValueError("endian must be big|little")
    if data_type in {"uint16", "u16"}:
        v = regs[0] & 0xFFFF
        if endian == "little":
            v = ((v & 0xFF) << 8) | ((v >> 8) & 0xFF)
        return str(v)
    if data_type in {"int16", "i16"}:
        v = regs[0] & 0xFFFF
        if endian == "little":
            v = ((v & 0xFF) << 8) | ((v >> 8) & 0xFF)
        v = struct.unpack(">h", struct.pack(">H", v))[0]
        return str(v)
    if data_type in {"uint32", "u32", "int32", "i32", "float32", "f32"}:
        if len(regs) < 2:
            return "need 2 registers"
        r0, r1 = regs[0] & 0xFFFF, regs[1] & 0xFFFF
        if endian == "little":
            r0 = ((r0 & 0xFF) << 8) | ((r0 >> 8) & 0xFF)
            r1 = ((r1 & 0xFF) << 8) | ((r1 >> 8) & 0xFF)
        raw = struct.pack(">HH", r0, r1)  # ABCD
        if word_order is None:
            return "wordOrder required for 32-bit decode"
        raw = apply_word_order_32(raw, parse_word_order(word_order))
        if data_type in {"uint32", "u32"}:
            return str(struct.unpack(">I", raw)[0])
        if data_type in {"int32", "i32"}:
            return str(struct.unpack(">i", raw)[0])
        return str(struct.unpack(">f", raw)[0])
    return "unsupported dataType (try uint16/int16/uint32/int32/float32)"


def cmd_read(args: argparse.Namespace) -> int:
    fc = args.function
    address = args.address
    quantity = args.quantity
    if address < 0 or address > 65535:
        raise ValueError("address must be 0..65535 (0-based offset)")
    if quantity <= 0 or quantity > 2000:
        raise ValueError("quantity must be 1..2000")

    pdu = struct.pack(">BHH", fc, address, quantity)
    _, rx_pdu = modbus_request(args.host, args.port, args.unit_id, pdu, args.timeout)
    if not rx_pdu:
        print("empty response")
        return 2
    if rx_pdu[0] == (fc | 0x80):
        print(decode_exception(fc, rx_pdu))
        return 2
    if rx_pdu[0] != fc:
        print(f"unexpected function code: 0x{rx_pdu[0]:02X}")
        return 2
    if fc in {1, 2}:
        # bits: byte count + packed coils
        if len(rx_pdu) < 2:
            print("truncated")
            return 2
        byte_count = rx_pdu[1]
        data = rx_pdu[2 : 2 + byte_count]
        bits = []
        for i in range(quantity):
            byte = data[i // 8] if i // 8 < len(data) else 0
            bits.append(1 if (byte >> (i % 8)) & 1 else 0)
        print("bits:", " ".join(map(str, bits)))
        return 0

    # registers: byte count + u16 words big-endian
    if len(rx_pdu) < 2:
        print("truncated")
        return 2
    byte_count = rx_pdu[1]
    data = rx_pdu[2 : 2 + byte_count]
    if byte_count % 2 != 0:
        print("invalid register byteCount")
        return 2
    regs = [struct.unpack(">H", data[i : i + 2])[0] for i in range(0, len(data), 2)]
    print("registers(hex):", " ".join(f"0x{r:04X}" for r in regs))
    if args.decode:
        decoded = decode_registers(regs, args.decode, args.endian, args.word_order)
        print("decoded:", decoded)
    return 0


def cmd_write(args: argparse.Namespace) -> int:
    fc = args.function
    address = args.address
    if address < 0 or address > 65535:
        raise ValueError("address must be 0..65535 (0-based offset)")

    if fc == 5:
        val = 0xFF00 if args.value.lower() in {"1", "true", "on"} else 0x0000
        pdu = struct.pack(">BHH", fc, address, val)
    elif fc == 6:
        v = int(args.value, 0)
        if v < 0 or v > 0xFFFF:
            raise ValueError("value must be 0..65535 for fc06")
        pdu = struct.pack(">BHH", fc, address, v)
    else:
        raise ValueError("write supports only fc05/fc06")

    _, rx_pdu = modbus_request(args.host, args.port, args.unit_id, pdu, args.timeout)
    if not rx_pdu:
        print("empty response")
        return 2
    if rx_pdu[0] == (fc | 0x80):
        print(decode_exception(fc, rx_pdu))
        return 2
    print("ok:", rx_pdu.hex())
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="modbus_tcp_tool.py",
        description="Modbus TCP smoke tool (no deps). Address is 0-based offset.",
    )
    p.add_argument("--host", required=True)
    p.add_argument("--port", type=int, default=502)
    p.add_argument("--unit-id", type=int, default=1)
    p.add_argument("--timeout", type=float, default=1.0)

    sub = p.add_subparsers(dest="cmd", required=True)

    r = sub.add_parser("read", help="read coils/inputs/registers")
    r.add_argument(
        "--function",
        type=int,
        choices=[1, 2, 3, 4],
        required=True,
        help="1/2/3/4",
    )
    r.add_argument("--address", type=int, required=True, help="0-based offset")
    r.add_argument("--quantity", type=int, required=True)
    r.add_argument("--decode", help="optional: uint16/int16/uint32/int32/float32")
    r.add_argument("--endian", default="big", help="big|little (for decode)")
    r.add_argument("--word-order", help="ABCD/CDAB/BADC/DCBA (for 32-bit decode)")
    r.set_defaults(func=cmd_read)

    w = sub.add_parser("write", help="write single coil/register")
    w.add_argument("--function", type=int, choices=[5, 6], required=True, help="5/6")
    w.add_argument("--address", type=int, required=True, help="0-based offset")
    w.add_argument("--value", required=True, help="fc05: on/off/1/0; fc06: 0..65535")
    w.set_defaults(func=cmd_write)

    return p


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    if not (0 <= args.unit_id <= 247):
        print("unit-id must be 0..247 (recommend 1..247)", file=sys.stderr)
        return 2
    try:
        return int(args.func(args))
    except Exception as e:
        print(f"error: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

