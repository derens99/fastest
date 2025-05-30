#!/usr/bin/env python3
"""Debug MessagePack communication"""

import msgpack
import json
import sys

# Test data that matches the WorkerCommand structure
test_cmd = {
    "id": 1,
    "tests": [{
        "id": "test_1",
        "module": "test_module",
        "func": "test_func",
        "path": "",
        "params": None
    }]
}

print("Testing MessagePack serialization/deserialization...")

# Test 1: Basic msgpack round-trip
print("1. Basic msgpack test:")
packed = msgpack.packb(test_cmd, use_bin_type=True)
unpacked = msgpack.unpackb(packed, raw=False)
print(f"   Original: {test_cmd}")
print(f"   Unpacked: {unpacked}")
print(f"   Match: {test_cmd == unpacked}")

# Test 2: Stream-based unpacking (like in the worker)
print("\n2. Stream-based unpacking test:")
import io
buffer = io.BytesIO(packed)
unpacker = msgpack.Unpacker(buffer, raw=False)
try:
    cmd = next(unpacker)
    print(f"   Unpacked from stream: {cmd}")
    print(f"   Match: {test_cmd == cmd}")
except Exception as e:
    print(f"   Error: {e}")

# Test 3: Compare with JSON
print("\n3. JSON comparison:")
json_str = json.dumps(test_cmd)
json_parsed = json.loads(json_str)
print(f"   JSON size: {len(json_str)} bytes")
print(f"   MessagePack size: {len(packed)} bytes")
print(f"   Size ratio: {len(packed)/len(json_str):.2f}")

print("\nMessagePack debug complete.")