"""FNV-1a + open addressing that matches the Ark NameIndex probe."""


def ark_i32(n: int) -> int:
    n &= 0xFFFFFFFF
    if n >= 0x80000000:
        n -= 0x100000000
    return n


def ark_fnv(text: str) -> int:
    hash_val = 216613626
    for ch in text:
        hash_val = ark_i32(ark_i32(hash_val * 16777619) ^ ord(ch))
        if hash_val < 0:
            hash_val = ark_i32(-hash_val)
    if hash_val < 0:
        hash_val = 0
    return hash_val


def hash_capacity(entry_count: int) -> int:
    capacity = 16
    required = entry_count * 2
    while capacity < required:
        capacity *= 2
    return capacity


def build_buckets(keys: list[str]) -> list[int]:
    capacity = hash_capacity(len(keys))
    buckets = [-1] * capacity
    for index, key in enumerate(keys):
        slot = ark_fnv(key) % capacity
        probes = 0
        while probes < capacity:
            if buckets[slot] < 0:
                buckets[slot] = index
                break
            slot = (slot + 1) % capacity
            probes += 1
        else:
            raise ValueError(f"hash table full while inserting {key!r}")
    return buckets


def emit_lookup_fn(fn_name: str, key_at_fn: str, bucket_fn: str, keys: list[str]) -> list[str]:
    buckets = build_buckets(keys)
    capacity = len(buckets)
    lines = [
        f"fn {bucket_fn}(slot: i32) -> i32 {{",
    ]
    for slot, index in enumerate(buckets):
        if index >= 0:
            lines.append(f"    if slot == {slot} {{ return {index} }}")
    lines.extend(
        [
            "    return 0 - 1",
            "}",
            "",
            f"fn {fn_name}(name: String) -> i32 {{",
            f"    let capacity = {capacity}",
            "    let fnv_offset = 216613626",
            "    let fnv_prime = 16777619",
            "    let mut hash = fnv_offset",
            "    let name_len = len(name)",
            "    let mut hi = 0",
            "    while hi < name_len {",
            "        hash = hash * fnv_prime ^ char_at(name, hi)",
            "        if hash < 0 {",
            "            hash = 0 - hash",
            "        }",
            "        hi = hi + 1",
            "    }",
            "    if hash < 0 {",
            "        hash = 0",
            "    }",
            "    let mut slot = hash % capacity",
            "    let mut probes = 0",
            "    while probes < capacity {",
            f"        let idx = {bucket_fn}(slot)",
            "        if idx < 0 {",
            "            return 0 - 1",
            "        }",
            f"        if eq(clone(name), {key_at_fn}(idx)) {{",
            "            return idx",
            "        }",
            "        slot = (slot + 1) % capacity",
            "        probes = probes + 1",
            "    }",
            "    return 0 - 1",
            "}",
        ]
    )
    return lines
