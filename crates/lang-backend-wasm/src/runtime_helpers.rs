use super::*;

pub(crate) fn emit_console_println_helper(abi: &WasmAbi, out: &mut String) {
    if abi.target == WasmTarget::JavaScriptHost {
        out.push_str("  (func $console.println (param $ptr i32)\n");
        out.push_str("    local.get $ptr\n");
        out.push_str("    local.get $ptr\n");
        out.push_str("    call $__strlen\n");
        out.push_str("    call $__host_console_println\n");
        out.push_str("  )\n");
        return;
    }

    let iovec_ptr = abi.iovec_base();
    let iovec_len = abi.iovec_base() + 4;
    let nwritten = abi.nwritten_base();
    let newline = abi.newline_base();

    out.push_str("  (func $console.println (param $ptr i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $cur i32)\n");
    // Compute strlen: scan for NUL byte
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.set $cur\n");
    out.push_str("    (block $strlen_break\n");
    out.push_str("      (loop $strlen_loop\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $strlen_break\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cur\n");
    out.push_str("        br $strlen_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cur\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $len\n");
    // Set iovec: {ptr, len}
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.store\n");
    out.push_str(&format!("    i32.const {iovec_len}\n"));
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    // fd_write(1, iovec_base, 1, nwritten)
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {nwritten}\n"));
    out.push_str("    call $fd_write\n");
    out.push_str("    drop\n");
    // Write newline: iovec = {newline_base, 1}
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str(&format!("    i32.const {newline}\n"));
    out.push_str("    i32.store\n");
    out.push_str(&format!("    i32.const {iovec_len}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {iovec_ptr}\n"));
    out.push_str("    i32.const 1\n");
    out.push_str(&format!("    i32.const {nwritten}\n"));
    out.push_str("    call $fd_write\n");
    out.push_str("    drop\n");
    out.push_str("  )\n");
}

/// Emit the `$string` helper: converts an i32 to decimal ASCII in the scratch
/// buffer, then copies it into durable heap-backed storage and returns a pointer.
pub(crate) fn emit_string_helper(abi: &WasmAbi, out: &mut String) {
    // str_buf occupies [scratch_base+16, scratch_base+28), written backward.
    // str_buf_end is the exclusive end; we start by placing NUL at str_buf_end-1.
    let nul_pos = abi.str_buf_end() - 1;

    out.push_str("  (func $string (param $n i32) (result i32)\n");
    out.push_str("    (local $abs i32)\n");
    out.push_str("    (local $neg i32)\n");
    out.push_str("    (local $pos i32)\n");
    out.push_str("    (local $src i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $dst i32)\n");
    // Write NUL at nul_pos
    out.push_str(&format!("    i32.const {nul_pos}\n"));
    out.push_str("    local.set $pos\n");
    out.push_str("    local.get $pos\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    // neg = (n < 0)
    out.push_str("    local.get $n\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    local.set $neg\n");
    // Special case: n == 0 → write '0'
    out.push_str("    local.get $n\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $pos\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.store8\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    // abs = neg ? (0 - n wrapping) : n
    // Wrapping subtraction is safe: for INT_MIN, (0 - INT_MIN) wraps to INT_MIN,
    // but i32.rem_u / i32.div_u treat it as unsigned 2147483648, giving correct digits.
    out.push_str("        local.get $neg\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.get $n\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $abs\n");
    out.push_str("          )\n");
    out.push_str("          (else\n");
    out.push_str("            local.get $n\n");
    out.push_str("            local.set $abs\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    // Loop: extract digits backward
    out.push_str("        (block $digits_break\n");
    out.push_str("          (loop $digits_loop\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $digits_break\n");
    out.push_str("            local.get $pos\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $pos\n");
    out.push_str("            local.get $pos\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.const 10\n");
    out.push_str("            i32.rem_u\n");
    out.push_str("            i32.const 48\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.store8\n");
    out.push_str("            local.get $abs\n");
    out.push_str("            i32.const 10\n");
    out.push_str("            i32.div_u\n");
    out.push_str("            local.set $abs\n");
    out.push_str("            br $digits_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    // Write '-' if negative
    out.push_str("    local.get $neg\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $pos\n");
    out.push_str("        local.get $pos\n");
    out.push_str("        i32.const 45\n");
    out.push_str("        i32.store8\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $pos\n");
    out.push_str("    local.set $src\n");
    out.push_str("    local.get $src\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $dst\n");
    out.push_str("    local.get $dst\n");
    out.push_str("    local.get $src\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $dst\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_fs_read_text_helper(abi: &WasmAbi, out: &mut String) -> Result<()> {
    let opened_fd = abi.fs_opened_fd_base();
    let iovec = abi.fs_iovec_base();
    let iovec_len = abi.fs_iovec_base() + 4;
    let nread = abi.fs_nread_base();
    let buffer = abi.fs_read_buffer_base();
    let buffer_len = abi.fs_read_buffer_len() - 1;
    let file_not_found_tag = abi.required_fieldless_variant_tag("FileNotFound")?;
    let permission_denied_tag = abi.required_fieldless_variant_tag("PermissionDenied")?;
    let unknown_read_error_tag = abi.required_fieldless_variant_tag("UnknownReadError")?;

    out.push_str("  (func $fs.read_text (param $path i32) (result i32)\n");
    out.push_str("    (local $path_len i32)\n");
    out.push_str("    (local $errno i32)\n");
    out.push_str("    (local $fd i32)\n");
    out.push_str("    (local $bytes_read i32)\n");
    out.push_str("    (local $text_ptr i32)\n");
    out.push_str("    (local $error_tag i32)\n");
    out.push_str("    (local $error_ptr i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    local.get $path\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $path_len\n");
    out.push_str("    i32.const 3\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    local.get $path\n");
    out.push_str("    local.get $path_len\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i64.const 2\n");
    out.push_str("    i64.const 0\n");
    out.push_str("    i32.const 0\n");
    out.push_str(&format!("    i32.const {opened_fd}\n"));
    out.push_str("    call $path_open\n");
    out.push_str("    local.set $errno\n");
    out.push_str("    local.get $errno\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str(&format!("        i32.const {opened_fd}\n"));
    out.push_str("        i32.load\n");
    out.push_str("        local.set $fd\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        i32.store\n");
    out.push_str(&format!("        i32.const {iovec_len}\n"));
    out.push_str(&format!("        i32.const {buffer_len}\n"));
    out.push_str("        i32.store\n");
    out.push_str("        local.get $fd\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str("        i32.const 1\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        call $fd_read\n");
    out.push_str("        local.set $errno\n");
    out.push_str("        local.get $fd\n");
    out.push_str("        call $fd_close\n");
    out.push_str("        drop\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if (result i32)\n");
    out.push_str("          (then\n");
    out.push_str(&format!("            i32.const {nread}\n"));
    out.push_str("            i32.load\n");
    out.push_str("            local.set $bytes_read\n");
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $text_ptr\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str(&format!("            i32.const {buffer}\n"));
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str("            local.get $bytes_read\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store8\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            local.get $text_ptr\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("          )\n");
    out.push_str("          (else\n");
    out.push_str(&format!("            i32.const {unknown_read_error_tag}\n"));
    out.push_str("            local.set $error_tag\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $error_ptr\n");
    out.push_str("            local.get $error_ptr\n");
    out.push_str("            local.get $error_tag\n");
    out.push_str("            i32.store\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            local.get $error_ptr\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.const 44\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if (result i32)\n");
    out.push_str(&format!(
        "          (then i32.const {file_not_found_tag})\n"
    ));
    out.push_str("          (else\n");
    out.push_str("            local.get $errno\n");
    out.push_str("            i32.const 2\n");
    out.push_str("            i32.eq\n");
    out.push_str("            (if (result i32)\n");
    out.push_str(&format!(
        "              (then i32.const {permission_denied_tag})\n"
    ));
    out.push_str(&format!(
        "              (else i32.const {unknown_read_error_tag})\n"
    ));
    out.push_str("            )\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.set $error_tag\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $error_ptr\n");
    out.push_str("        local.get $error_ptr\n");
    out.push_str("        local.get $error_tag\n");
    out.push_str("        i32.store\n");
    out.push_str("        i32.const 8\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        local.get $error_ptr\n");
    out.push_str("        i32.store offset=4\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_stdin_read_text_helper(abi: &WasmAbi, out: &mut String) {
    let iovec = abi.fs_iovec_base();
    let iovec_len = abi.fs_iovec_base() + 4;
    let nread = abi.fs_nread_base();
    let buffer = abi.fs_read_buffer_base();
    let buffer_len = abi.fs_read_buffer_len() - 1;

    out.push_str("  (func $stdin.read_text (result i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $chunk_len i32)\n");
    out.push_str("    (local $new_ptr i32)\n");
    out.push_str("    (local $new_len i32)\n");
    out.push_str("    (local $errno i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        i32.store\n");
    out.push_str(&format!("        i32.const {iovec_len}\n"));
    out.push_str(&format!("        i32.const {buffer_len}\n"));
    out.push_str("        i32.store\n");
    out.push_str("        i32.const 0\n");
    out.push_str(&format!("        i32.const {iovec}\n"));
    out.push_str("        i32.const 1\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        call $fd_read\n");
    out.push_str("        local.set $errno\n");
    out.push_str("        local.get $errno\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if\n");
    out.push_str("          (then)\n");
    out.push_str("          (else unreachable)\n");
    out.push_str("        )\n");
    out.push_str(&format!("        i32.const {nread}\n"));
    out.push_str("        i32.load\n");
    out.push_str("        local.set $chunk_len\n");
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $len\n");
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $new_len\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $new_ptr\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.add\n");
    out.push_str(&format!("        i32.const {buffer}\n"));
    out.push_str("        local.get $chunk_len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $new_ptr\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $new_len\n");
    out.push_str("        local.set $len\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_stdin_read_line_helper(out: &mut String) {
    out.push_str("  (global $__stdin_line_text_ptr (mut i32) (i32.const 0))\n");
    out.push_str("  (global $__stdin_line_pos (mut i32) (i32.const 0))\n");
    out.push_str("  (func $stdin.read_line (result i32)\n");
    out.push_str("    (local $text_ptr i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $copy_len i32)\n");
    out.push_str("    (local $line_ptr i32)\n");
    out.push_str("    global.get $__stdin_line_text_ptr\n");
    out.push_str("    local.tee $text_ptr\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        call $stdin.read_text\n");
    out.push_str("        local.tee $text_ptr\n");
    out.push_str("        global.set $__stdin_line_text_ptr\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    global.get $__stdin_line_pos\n");
    out.push_str("    local.tee $start\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $scan_done\n");
    out.push_str("      (loop $scan_loop\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $scan_done\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.eq\n");
    out.push_str("        br_if $scan_done\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $scan_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    local.set $copy_len\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.gt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text_ptr\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 13\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $copy_len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $copy_len\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $line_ptr\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("    local.get $text_ptr\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.add\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("    local.get $copy_len\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $text_ptr\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        global.set $__stdin_line_pos\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $line_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_ascii_whitespace_helper(out: &mut String) {
    out.push_str("  (func $__is_ascii_whitespace (param $byte i32) (result i32)\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 32\n");
    out.push_str("    i32.eq\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 9\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 10\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 11\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 12\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("    local.get $byte\n");
    out.push_str("    i32.const 13\n");
    out.push_str("    i32.eq\n");
    out.push_str("    i32.or\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_split_whitespace_helper(out: &mut String) {
    out.push_str("  (func $split_whitespace (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $count i32)\n");
    out.push_str("    (local $items_ptr i32)\n");
    out.push_str("    (local $list_ptr i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $token_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $count_done\n");
    out.push_str("      (loop $count_loop\n");
    out.push_str("        (block $skip_done\n");
    out.push_str("          (loop $skip_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $count_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $skip_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $count\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $count\n");
    out.push_str("        (block $token_done\n");
    out.push_str("          (loop $token_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $count_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $token_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        br $count_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $count\n");
    out.push_str("    i32.eqz\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then i32.const 4)\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $count\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items_ptr\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $emit_done\n");
    out.push_str("      (loop $emit_loop\n");
    out.push_str("        (block $emit_skip_done\n");
    out.push_str("          (loop $emit_skip_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $emit_skip_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.set $start\n");
    out.push_str("        (block $emit_token_done\n");
    out.push_str("          (loop $emit_token_loop\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $emit_token_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $emit_token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $emit_token_loop\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $len\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $token_ptr\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        local.get $start\n");
    out.push_str("        local.get $len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $items_ptr\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $emit_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $list_ptr\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("    local.get $count\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("    local.get $items_ptr\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $list_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_split_whitespace_nth_helper(out: &mut String) {
    out.push_str(
        "  (func $__split_whitespace_nth (param $text i32) (param $target i32) (result i32)\n",
    );
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $token_ptr i32)\n");
    out.push_str("    local.get $target\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.tee $token_ptr\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $token_ptr\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $outer\n");
    out.push_str("        (block $skip_done\n");
    out.push_str("          (loop $skip\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $skip_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $skip\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.set $start\n");
    out.push_str("        (block $token_done\n");
    out.push_str("          (loop $token\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.load8_u\n");
    out.push_str("            local.tee $byte\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $byte\n");
    out.push_str("            call $__is_ascii_whitespace\n");
    out.push_str("            br_if $token_done\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $scan\n");
    out.push_str("            br $token\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $target\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $scan\n");
    out.push_str("            local.get $start\n");
    out.push_str("            i32.sub\n");
    out.push_str("            local.set $len\n");
    out.push_str("            local.get $len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $token_ptr\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            local.get $start\n");
    out.push_str("            local.get $len\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            local.get $len\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store8\n");
    out.push_str("            local.get $token_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $outer\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.tee $token_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $token_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_strip_suffix_helper(out: &mut String) {
    out.push_str("  (func $strip_suffix (param $text i32) (param $suffix i32) (result i32)\n");
    out.push_str("    (local $text_len i32)\n");
    out.push_str("    (local $suffix_len i32)\n");
    out.push_str("    (local $rest_len i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    (local $rest_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $text_len\n");
    out.push_str("    local.get $suffix\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $suffix_len\n");
    out.push_str("    local.get $text_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.lt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $result_ptr\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $result_ptr\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $text_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $rest_len\n");
    out.push_str("    (block $mismatch\n");
    out.push_str("      (loop $compare\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        local.get $suffix_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $mismatch\n");
    out.push_str("        local.get $text\n");
    out.push_str("        local.get $rest_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $suffix\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $compare\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $rest_ptr\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    call $__memcpy\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    local.get $rest_len\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    local.get $rest_ptr\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_option_unwrap_or_helper(out: &mut String) {
    out.push_str(
        "  (func $__option_unwrap_or (param $option i32) (param $fallback i32) (result i32)\n",
    );
    out.push_str("    local.get $option\n");
    out.push_str("    i32.load\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $option\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $fallback\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_parse_i64_or_zero_helper(out: &mut String) {
    out.push_str("  (func $__parse_i64_or_zero (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    (local $sign i32)\n");
    out.push_str("    (local $has_digits i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $sign\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.const 45\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        local.set $sign\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $invalid\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.set $byte\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $has_digits\n");
    out.push_str("            i32.eqz\n");
    out.push_str("            br_if $invalid\n");
    out.push_str("            local.get $sign\n");
    out.push_str("            i32.const -1\n");
    out.push_str("            i32.eq\n");
    out.push_str("            (if (result i32)\n");
    out.push_str("              (then\n");
    out.push_str("                i32.const 0\n");
    out.push_str("                local.get $value\n");
    out.push_str("                i32.sub\n");
    out.push_str("              )\n");
    out.push_str("              (else\n");
    out.push_str("                local.get $value\n");
    out.push_str("              )\n");
    out.push_str("            )\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        br_if $invalid\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 57\n");
    out.push_str("        i32.gt_u\n");
    out.push_str("        br_if $invalid\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        local.set $has_digits\n");
    out.push_str("        local.get $value\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.mul\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 0\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_parse_i64_helper(_abi: &WasmAbi, out: &mut String) {
    out.push_str("  (func $parse.i64 (param $text i32) (result i32)\n");
    out.push_str("    (local $scan i32)\n");
    out.push_str("    (local $byte i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    (local $sign i32)\n");
    out.push_str("    (local $has_digits i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $sign\n");
    out.push_str("    local.get $text\n");
    out.push_str("    local.set $scan\n");
    out.push_str("    local.get $scan\n");
    out.push_str("    i32.load8_u\n");
    out.push_str("    i32.const 45\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        local.set $sign\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $parse_err\n");
    out.push_str("      (block $parse_done\n");
    out.push_str("        (loop $parse_loop\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.tee $byte\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $parse_done\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        br_if $parse_err\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 57\n");
    out.push_str("        i32.gt_u\n");
    out.push_str("        br_if $parse_err\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        local.set $has_digits\n");
    out.push_str("        local.get $value\n");
    out.push_str("        i32.const 10\n");
    out.push_str("        i32.mul\n");
    out.push_str("        local.get $byte\n");
    out.push_str("        i32.const 48\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $scan\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $scan\n");
    out.push_str("        br $parse_loop\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("      local.get $has_digits\n");
    out.push_str("      i32.eqz\n");
    out.push_str("      br_if $parse_err\n");
    out.push_str("      i32.const 8\n");
    out.push_str("      call $__alloc\n");
    out.push_str("      local.set $result_ptr\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      i32.const 0\n");
    out.push_str("      i32.store\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      local.get $value\n");
    out.push_str("      local.get $sign\n");
    out.push_str("      i32.mul\n");
    out.push_str("      i32.store offset=4\n");
    out.push_str("      local.get $result_ptr\n");
    out.push_str("      return\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_parse_bool_helper(_abi: &WasmAbi, out: &mut String) {
    out.push_str("  (func $parse.bool (param $text i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $result_ptr i32)\n");
    out.push_str("    local.get $text\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 116\n");
    out.push_str("        i32.eq\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=1\n");
    out.push_str("        i32.const 114\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=2\n");
    out.push_str("        i32.const 117\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=3\n");
    out.push_str("        i32.const 101\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 5\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.const 102\n");
    out.push_str("        i32.eq\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=1\n");
    out.push_str("        i32.const 97\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=2\n");
    out.push_str("        i32.const 108\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=3\n");
    out.push_str("        i32.const 115\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        local.get $text\n");
    out.push_str("        i32.load8_u offset=4\n");
    out.push_str("        i32.const 101\n");
    out.push_str("        i32.eq\n");
    out.push_str("        i32.and\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 8\n");
    out.push_str("            call $__alloc\n");
    out.push_str("            local.set $result_ptr\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            i32.store offset=4\n");
    out.push_str("            local.get $result_ptr\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $result_ptr\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $result_ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_alloc_helper(out: &mut String) {
    out.push_str("  (func $__alloc (param $size i32) (result i32)\n");
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    (local $aligned i32)\n");
    out.push_str("    (local $needed_end i32)\n");
    out.push_str("    (local $current_bytes i32)\n");
    out.push_str("    (local $grow_bytes i32)\n");
    out.push_str("    (local $grow_pages i32)\n");
    out.push_str("    global.get $heap_ptr\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $size\n");
    out.push_str("    i32.const 3\n");
    out.push_str("    i32.add\n");
    out.push_str("    i32.const -4\n");
    out.push_str("    i32.and\n");
    out.push_str("    local.set $aligned\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.get $aligned\n");
    out.push_str("    i32.add\n");
    out.push_str("    local.set $needed_end\n");
    out.push_str("    memory.size\n");
    out.push_str("    i32.const 16\n");
    out.push_str("    i32.shl\n");
    out.push_str("    local.set $current_bytes\n");
    out.push_str("    local.get $needed_end\n");
    out.push_str("    local.get $current_bytes\n");
    out.push_str("    i32.gt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        local.get $needed_end\n");
    out.push_str("        local.get $current_bytes\n");
    out.push_str("        i32.sub\n");
    out.push_str("        local.set $grow_bytes\n");
    out.push_str("        local.get $grow_bytes\n");
    out.push_str("        i32.const 65535\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.const 16\n");
    out.push_str("        i32.shr_u\n");
    out.push_str("        local.set $grow_pages\n");
    out.push_str("        local.get $grow_pages\n");
    out.push_str("        memory.grow\n");
    out.push_str("        i32.const -1\n");
    out.push_str("        i32.eq\n");
    out.push_str("        (if\n");
    out.push_str("          (then unreachable)\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $needed_end\n");
    out.push_str("    global.set $heap_ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_strlen_helper(out: &mut String) {
    out.push_str("  (func $__strlen (param $ptr i32) (result i32)\n");
    out.push_str("    (local $cur i32)\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    local.set $cur\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.eqz\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $cur\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cur\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cur\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("    i32.sub\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_string_eq_helper(out: &mut String) {
    out.push_str("  (func $__streq (param $left i32) (param $right i32) (result i32)\n");
    out.push_str("    (local $left_len i32)\n");
    out.push_str("    (local $right_len i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (local $equal i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $equal\n");
    out.push_str("    local.get $left\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $left_len\n");
    out.push_str("    local.get $right\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $right_len\n");
    out.push_str("    local.get $left_len\n");
    out.push_str("    local.get $right_len\n");
    out.push_str("    i32.ne\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $left_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $left\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $right\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.set $equal\n");
    out.push_str("            br $done\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $equal\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_ends_with_at_helper(out: &mut String) {
    out.push_str("  (func $ends_with_at (param $text i32) (param $suffix i32) (param $end i32) (result i32)\n");
    out.push_str("    (local $suffix_len i32)\n");
    out.push_str("    (local $start i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (local $matches i32)\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    local.set $matches\n");
    out.push_str("    local.get $end\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $suffix\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $suffix_len\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    local.get $end\n");
    out.push_str("    i32.gt_u\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        return\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $end\n");
    out.push_str("    local.get $suffix_len\n");
    out.push_str("    i32.sub\n");
    out.push_str("    local.set $start\n");
    out.push_str("    (block $done\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $suffix_len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $done\n");
    out.push_str("        local.get $text\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        local.get $suffix\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 0\n");
    out.push_str("            local.set $matches\n");
    out.push_str("            br $done\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $matches\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_memcpy_helper(out: &mut String) {
    out.push_str("  (func $__memcpy (param $dst i32) (param $src i32) (param $len i32)\n");
    out.push_str("    (local $i i32)\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $i\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $dst\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $src\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load8_u\n");
    out.push_str("        i32.store8\n");
    out.push_str("        local.get $i\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $i\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_list_get_helper(out: &mut String) {
    out.push_str("  (func $__list_get (param $list i32) (param $index i32) (result i32)\n");
    out.push_str("    local.get $index\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then i32.const 0)\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $list\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        (if (result i32)\n");
    out.push_str("          (then i32.const 0)\n");
    out.push_str("          (else\n");
    out.push_str("            local.get $list\n");
    out.push_str("            i32.load offset=4\n");
    out.push_str("            local.get $index\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            i32.mul\n");
    out.push_str("            i32.add\n");
    out.push_str("            i32.load\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_closure_allocator(
    abi: &WasmAbi,
    closure: &ClosureThunk,
    out: &mut String,
) -> Result<()> {
    out.push_str(&format!("  (func ${}", closure.alloc_name));
    for capture in &closure.captures {
        let wasm_ty = abi
            .wasm_type(&capture.ty)?
            .ok_or_else(|| anyhow!("unsupported unit capture in wasm backend"))?;
        out.push_str(&format!(" (param ${} {wasm_ty})", capture.name));
    }
    out.push_str(" (result i32)\n");
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    global.get $heap_ptr\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", closure.env_size));
    out.push_str("    i32.add\n");
    out.push_str("    global.set $heap_ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", closure.table_index));
    out.push_str("    i32.store\n");
    for capture in &closure.captures {
        out.push_str("    local.get $ptr\n");
        out.push_str(&format!("    local.get ${}\n", capture.name));
        out.push_str(&format!("    i32.store offset={}\n", capture.offset));
    }
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_named_callback_allocator(out: &mut String, callback: &NamedCallbackThunk) {
    out.push_str(&format!("  (func ${} (result i32)\n", callback.alloc_name));
    out.push_str("    (local $ptr i32)\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $ptr\n");
    out.push_str("    local.get $ptr\n");
    out.push_str(&format!("    i32.const {}\n", callback.table_index));
    out.push_str("    i32.store\n");
    out.push_str("    local.get $ptr\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_apply_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported closure argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $closure i32) (param $arg {arg_ty})",
        abi.apply_helper_name(signature.index)
    ));
    if let Some(result_ty) = abi.wasm_type(&signature.key.result)? {
        out.push_str(&format!(" (result {result_ty})"));
    }
    out.push('\n');
    out.push_str("    local.get $closure\n");
    out.push_str("    local.get $arg\n");
    out.push_str("    local.get $closure\n");
    out.push_str("    i32.load\n");
    out.push_str(&format!(
        "    call_indirect (type ${})\n",
        abi.closure_type_name(signature.index)
    ));
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_apply_dynamic_helper(abi: &WasmAbi, out: &mut String) -> Result<()> {
    let signature = abi
        .closure_signatures
        .first()
        .ok_or_else(|| anyhow!("iterator lowering requires at least one closure signature"))?;
    out.push_str(
        "  (func $__apply_closure_dyn (param $closure i32) (param $arg i32) (result i32)\n",
    );
    out.push_str("    local.get $closure\n");
    out.push_str("    local.get $arg\n");
    out.push_str("    local.get $closure\n");
    out.push_str("    i32.load\n");
    out.push_str(&format!(
        "    call_indirect (type ${})\n",
        abi.closure_type_name(signature.index)
    ));
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_take_helper(out: &mut String) {
    out.push_str("  (func $__iter_take_i64 (param $iter i32) (param $limit i32) (result i32)\n");
    out.push_str("    (local $state i32)\n");
    out.push_str("    (local $callback i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $list i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $step i32)\n");
    out.push_str("    local.get $limit\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $state\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $callback\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $len\n");
    out.push_str("        local.get $limit\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $state\n");
    out.push_str("        call $__apply_closure_dyn\n");
    out.push_str("        local.set $step\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.eq\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $step\n");
    out.push_str("        i32.load offset=8\n");
    out.push_str("        local.set $state\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $len\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $list\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $list\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_map_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported map argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.map_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $in_items i32)\n");
    out.push_str("    (local $out_items i32)\n");
    out.push_str("    (local $out_list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $in_items\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $in_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $out_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $value\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_list\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_option_map_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported option map argument type in wasm backend"))?;
    let _ = abi
        .wasm_type(&signature.key.result)?
        .ok_or_else(|| anyhow!("unsupported option map result type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $option i32) (param $callback i32) (result i32)\n",
        abi.option_map_helper_name(signature.index)
    ));
    out.push_str("    (local $out i32)\n");
    out.push_str("    local.get $option\n");
    out.push_str("    i32.load\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.eq\n");
    out.push_str("    (if (result i32)\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 8\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $out\n");
    out.push_str("        local.get $out\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $out\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $option\n");
    out.push_str("        i32.load offset=4\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        i32.store offset=4\n");
    out.push_str("        local.get $out\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        call $__alloc\n");
    out.push_str("        local.set $out\n");
    out.push_str("        local.get $out\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $out\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_any_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let _ = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported any argument type in wasm backend"))?;
    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.any_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $value\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        i32.const 0\n");
    out.push_str("        i32.ne\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            return\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 0\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_filter_helper(
    abi: &WasmAbi,
    signature: &ClosureSignature,
    out: &mut String,
) -> Result<()> {
    let arg_ty = abi
        .wasm_type(&signature.key.arg)?
        .ok_or_else(|| anyhow!("unsupported filter argument type in wasm backend"))?;
    if arg_ty != WasmTypeRepr::I32 || signature.key.result != Type::Bool {
        return Ok(());
    }

    out.push_str(&format!(
        "  (func ${} (param $list i32) (param $callback i32) (result i32)\n",
        abi.filter_helper_name(signature.index)
    ));
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $in_items i32)\n");
    out.push_str("    (local $out_items i32)\n");
    out.push_str("    (local $out_list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $out_len i32)\n");
    out.push_str("    (local $value i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $in_items\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $in_items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $value\n");
    out.push_str("        local.get $callback\n");
    out.push_str("        local.get $value\n");
    out.push_str(&format!(
        "        call ${}\n",
        abi.apply_helper_name(signature.index)
    ));
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $out_items\n");
    out.push_str("            local.get $out_len\n");
    out.push_str("            i32.const 4\n");
    out.push_str("            i32.mul\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.get $value\n");
    out.push_str("            i32.store\n");
    out.push_str("            local.get $out_len\n");
    out.push_str("            i32.const 1\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $out_len\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out_list\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("    local.get $out_items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $out_list\n");
    out.push_str("  )\n");
    Ok(())
}

pub(crate) fn emit_named_callback_thunk(callback: &NamedCallbackThunk, out: &mut String) {
    out.push_str(&format!(
        "  (func ${} (param $env i32) (param $value i32) (result i32)\n",
        callback.func_name
    ));
    match &callback.target {
        NamedCallbackTarget::Function(name) => {
            out.push_str("    local.get $value\n");
            out.push_str(&format!("    call ${name}\n"));
        }
        NamedCallbackTarget::BuiltinString => {
            out.push_str("    local.get $value\n");
            out.push_str("    call $string\n");
        }
    }
    out.push_str("  )\n");
}

pub(crate) fn emit_sum_helper(out: &mut String) {
    out.push_str("  (func $__list_sum_i64 (param $list i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $sum i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $sum\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $sum\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $sum\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_range_inclusive_helper(out: &mut String) {
    out.push_str("  (func $__range_inclusive (param $start i32) (param $end i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $list i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    local.get $end\n");
    out.push_str("    local.get $start\n");
    out.push_str("    i32.lt_s\n");
    out.push_str("    (if\n");
    out.push_str("      (then\n");
    out.push_str("        i32.const 0\n");
    out.push_str("        local.set $len\n");
    out.push_str("      )\n");
    out.push_str("      (else\n");
    out.push_str("        local.get $end\n");
    out.push_str("        local.get $start\n");
    out.push_str("        i32.sub\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $len\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.const 4\n");
    out.push_str("    i32.mul\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $items\n");
    out.push_str("    (block $break\n");
    out.push_str("      (loop $loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $start\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.store\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $list\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $len\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $list\n");
    out.push_str("    local.get $items\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $list\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_iter_unfold_helper(out: &mut String) {
    out.push_str(
        "  (func $__iter_unfold_new (param $state i32) (param $callback i32) (result i32)\n",
    );
    out.push_str("    (local $iter i32)\n");
    out.push_str("    i32.const 8\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $iter\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    local.get $state\n");
    out.push_str("    i32.store\n");
    out.push_str("    local.get $iter\n");
    out.push_str("    local.get $callback\n");
    out.push_str("    i32.store offset=4\n");
    out.push_str("    local.get $iter\n");
    out.push_str("  )\n");
}

pub(crate) fn emit_join_helper(out: &mut String) {
    out.push_str("  (func $__list_join_strings (param $list i32) (param $sep i32) (result i32)\n");
    out.push_str("    (local $len i32)\n");
    out.push_str("    (local $items i32)\n");
    out.push_str("    (local $sep_len i32)\n");
    out.push_str("    (local $total i32)\n");
    out.push_str("    (local $index i32)\n");
    out.push_str("    (local $item_ptr i32)\n");
    out.push_str("    (local $item_len i32)\n");
    out.push_str("    (local $out i32)\n");
    out.push_str("    (local $cursor i32)\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load\n");
    out.push_str("    local.set $len\n");
    out.push_str("    local.get $list\n");
    out.push_str("    i32.load offset=4\n");
    out.push_str("    local.set $items\n");
    out.push_str("    local.get $sep\n");
    out.push_str("    call $__strlen\n");
    out.push_str("    local.set $sep_len\n");
    out.push_str("    (block $measure_break\n");
    out.push_str("      (loop $measure_loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $measure_break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $item_ptr\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        call $__strlen\n");
    out.push_str("        local.set $item_len\n");
    out.push_str("        local.get $total\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $total\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $total\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $total\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $measure_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $total\n");
    out.push_str("    i32.const 1\n");
    out.push_str("    i32.add\n");
    out.push_str("    call $__alloc\n");
    out.push_str("    local.set $out\n");
    out.push_str("    local.get $out\n");
    out.push_str("    local.set $cursor\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    local.set $index\n");
    out.push_str("    (block $copy_break\n");
    out.push_str("      (loop $copy_loop\n");
    out.push_str("        local.get $index\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.ge_u\n");
    out.push_str("        br_if $copy_break\n");
    out.push_str("        local.get $items\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 4\n");
    out.push_str("        i32.mul\n");
    out.push_str("        i32.add\n");
    out.push_str("        i32.load\n");
    out.push_str("        local.set $item_ptr\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        call $__strlen\n");
    out.push_str("        local.set $item_len\n");
    out.push_str("        local.get $cursor\n");
    out.push_str("        local.get $item_ptr\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        call $__memcpy\n");
    out.push_str("        local.get $cursor\n");
    out.push_str("        local.get $item_len\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $cursor\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.get $len\n");
    out.push_str("        i32.lt_u\n");
    out.push_str("        (if\n");
    out.push_str("          (then\n");
    out.push_str("            local.get $cursor\n");
    out.push_str("            local.get $sep\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            call $__memcpy\n");
    out.push_str("            local.get $cursor\n");
    out.push_str("            local.get $sep_len\n");
    out.push_str("            i32.add\n");
    out.push_str("            local.set $cursor\n");
    out.push_str("          )\n");
    out.push_str("        )\n");
    out.push_str("        local.get $index\n");
    out.push_str("        i32.const 1\n");
    out.push_str("        i32.add\n");
    out.push_str("        local.set $index\n");
    out.push_str("        br $copy_loop\n");
    out.push_str("      )\n");
    out.push_str("    )\n");
    out.push_str("    local.get $cursor\n");
    out.push_str("    i32.const 0\n");
    out.push_str("    i32.store8\n");
    out.push_str("    local.get $out\n");
    out.push_str("  )\n");
}
