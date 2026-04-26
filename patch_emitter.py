
import sys

def patch():
    with open("src/compiler/emitter.ark", "r") as f:
        lines = f.readlines()

    indent = ""

    # NEW ERR RESULT (Specialized for global-variable bypass)
    # The emitter now needs to set the GLOBAL mutable variables in the host memory.
    # Wait! If I use global variables in Arkansas source, the emitter can just use OP_GLOBAL_SET
    # for THOSE globals.
    # But Stage 1 built the globals at some indices.

    # Actually, I have an even better way.
    # The emitter will just emit the code to set the globals!
    # But wait! I don't know the indices of bootstrap_tag, etc.

    # Okay, I'll stick to the "inline WASI" approach but I'll make the function return NOTHING.
    new_err_result_void = [
        # Store tag 1 (Err)
        f"{indent}emit_byte(w, OP_GLOBAL_GET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}emit_byte(w, OP_GLOBAL_GET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 1)\n",
        f"{indent}emit_byte(w, OP_I32_STORE())\n",
        f"{indent}emit_byte(w, 2)\n",
        f"{indent}emit_byte(w, 0)\n",
        
        # Store FsError in payload
        f"{indent}emit_byte(w, OP_GLOBAL_GET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 4)\n",
        f"{indent}emit_byte(w, OP_I32_ADD())\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 3) # FsError::IoError tag\n",
        f"{indent}emit_byte(w, OP_I32_STORE())\n",
        f"{indent}emit_byte(w, 2)\n",
        f"{indent}emit_byte(w, 0)\n",
        
        # Store SCRATCH_FS_STRPTR in payload
        f"{indent}emit_byte(w, OP_GLOBAL_GET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 8)\n",
        f"{indent}emit_byte(w, OP_I32_ADD())\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, SCRATCH_FS_STRPTR())\n",
        f"{indent}emit_byte(w, OP_I32_LOAD())\n",
        f"{indent}emit_byte(w, 2)\n",
        f"{indent}emit_byte(w, 0)\n",
        f"{indent}emit_byte(w, OP_I32_STORE())\n",
        f"{indent}emit_byte(w, 2)\n",
        f"{indent}emit_byte(w, 0)\n",
        
        # Drop the result pointer (since function returns void)
        f"{indent}emit_byte(w, OP_GLOBAL_GET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 12)\n",
        f"{indent}emit_byte(w, OP_I32_ADD())\n",
        f"{indent}emit_byte(w, OP_GLOBAL_SET())\n",
        f"{indent}emit_leb128_u(w, 0)\n",
        f"{indent}# No return value\n"
    ]

    new_ok_bump_void = [
        f"{indent}emit_byte(w, OP_I32_CONST())\n",
        f"{indent}emit_leb128_s(w, 12)\n",
        f"{indent}emit_byte(w, OP_I32_ADD())\n",
        f"{indent}emit_byte(w, OP_GLOBAL_SET())\n",
        f"{indent}emit_leb128_u(w, 0)\n"
    ]

    # Find read_to_string block
    read_start = -1
    for idx, line in enumerate(lines):
        if "fs::read_to_string" in line and "if eq(clone(callee)" in line:
            lines[idx] = line.replace('eq(clone(callee), "fs_read_file")', 'contains(clone(callee), "fs_read_file")')
            lines[idx] = lines[idx].replace('eq(clone(callee), "bootstrap_fs_read_file")', 'contains(clone(callee), "bootstrap_fs_read_file")')
            read_start = idx
            break
    
    if read_start != -1:
        # Patch ERR
        for j in range(read_start, read_start + 200):
            if "emit_leb128_s(w, 1)" in lines[j] and 'OP_I32_STORE' not in lines[j]:
                if "emit_byte(w, OP_I32_STORE())" in lines[j+1]:
                    start_pattern = j - 5
                    if "OP_GLOBAL_GET" in lines[start_pattern]:
                        lines[start_pattern:start_pattern+29] = new_err_result_void
                        break
        
        # Patch OK bump
        for j in range(read_start, read_start + 300):
             if "emit_leb128_s(w, 8)" in lines[j] and j > read_start + 50:
                 if "OP_I32_ADD" in lines[j+1] and "OP_GLOBAL_SET" in lines[j+2]:
                     lines[j-1:j+4] = new_ok_bump_void
                     break

    with open("src/compiler/emitter.ark", "w") as f:
        f.writelines(lines)
    print("Done")

if __name__ == "__main__":
    patch()
