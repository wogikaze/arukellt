#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo<'_>) -> ! {
    loop {}
}

#[link(wasm_import_module = "host")]
unsafe extern "C" {
    #[link_name = "runtime-buffer-new"]
    fn host_buffer_new() -> i32;
    #[link_name = "runtime-buffer-push"]
    fn host_buffer_push(handle: i32, byte: i32);
    #[link_name = "runtime-buffer-len"]
    fn host_buffer_len(handle: i32) -> i32;
    #[link_name = "runtime-buffer-byte"]
    fn host_buffer_byte(handle: i32, index: i32) -> i32;
    #[link_name = "runtime-buffer-close"]
    fn host_buffer_close(handle: i32);

    #[link_name = "runtime-http-get"]
    fn host_http_get(url_ptr: i32, url_len: i32) -> i32;
    #[link_name = "runtime-http-request"]
    fn host_http_request(method_ptr: i32, method_len: i32, url_ptr: i32, url_len: i32, body_ptr: i32, body_len: i32) -> i32;
    #[link_name = "runtime-http-serve"]
    fn host_http_serve(port: i32, body_ptr: i32, body_len: i32) -> i32;

    #[link_name = "runtime-socket-connect"]
    fn host_socket_connect(host_ptr: i32, host_len: i32, port: i32) -> i32;
    #[link_name = "runtime-socket-read"]
    fn host_socket_read(socket: i32, max_len: i32) -> i32;
    #[link_name = "runtime-socket-write"]
    fn host_socket_write(socket: i32, buffer: i32) -> i32;
    #[link_name = "runtime-socket-listen"]
    fn host_socket_listen(host_ptr: i32, host_len: i32, port: i32) -> i32;
    #[link_name = "runtime-socket-accept"]
    fn host_socket_accept(listener: i32) -> i32;
    #[link_name = "runtime-socket-close"]
    fn host_socket_close(handle: i32);

    #[link_name = "runtime-fs-read-file"]
    fn host_fs_read_file(path_ptr: i32, path_len: i32) -> i32;
    #[link_name = "runtime-fs-write-bytes"]
    fn host_fs_write_bytes(path_ptr: i32, path_len: i32, buffer: i32) -> i32;
    #[link_name = "runtime-fs-read-dir"]
    fn host_fs_read_dir(path_ptr: i32, path_len: i32) -> i32;
    #[link_name = "runtime-fs-metadata"]
    fn host_fs_metadata(path_ptr: i32, path_len: i32) -> i32;
    #[link_name = "runtime-fs-remove-file"]
    fn host_fs_remove_file(path_ptr: i32, path_len: i32) -> i32;
    #[link_name = "runtime-fs-create-dir-all"]
    fn host_fs_create_dir_all(path_ptr: i32, path_len: i32) -> i32;

    #[link_name = "runtime-env-vars"]
    fn host_env_vars() -> i32;
    #[link_name = "runtime-env-current-dir"]
    fn host_env_current_dir() -> i32;
}

static mut FS_PATH_PTR: i32 = 0;
static mut FS_PATH_LEN: i32 = 0;

#[inline]
unsafe fn load_i32(ptr: i32) -> i32 {
    unsafe { (ptr as *const i32).read_unaligned() }
}

#[inline]
unsafe fn store_i32(ptr: i32, value: i32) {
    unsafe { (ptr as *mut i32).write_unaligned(value) }
}

#[inline]
unsafe fn load_u8(ptr: i32) -> i32 {
    unsafe { (ptr as *const u8).read() as i32 }
}

#[inline]
unsafe fn store_u8(ptr: i32, value: i32) {
    unsafe { (ptr as *mut u8).write(value as u8) }
}

unsafe fn copy_handle_payload(signed_handle: i32, dst: i32) -> i32 {
    if signed_handle == 0 {
        return 0;
    }
    let negative = signed_handle < 0;
    let handle = if negative { -signed_handle } else { signed_handle };
    let len = unsafe { host_buffer_len(handle) };
    let mut i = 0;
    while i < len {
        let byte = unsafe { host_buffer_byte(handle, i) };
        unsafe { store_u8(dst + i, byte) };
        i += 1;
    }
    unsafe { host_buffer_close(handle) };
    if negative { -len } else { len }
}

unsafe fn scalar_or_error(value: i32, resp_ptr: i32) -> i32 {
    if value >= 0 {
        value
    } else {
        unsafe { copy_handle_payload(value, resp_ptr) }
    }
}

unsafe fn buffer_from_memory(ptr: i32, len: i32) -> i32 {
    let handle = unsafe { host_buffer_new() };
    let mut i = 0;
    while i < len {
        unsafe { host_buffer_push(handle, load_u8(ptr + i)) };
        i += 1;
    }
    handle
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_http_get(url_ptr: i32, url_len: i32, resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_http_get(url_ptr, url_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_http_request(method_ptr: i32, method_len: i32, url_ptr: i32, url_len: i32, body_ptr: i32, body_len: i32, resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_http_request(method_ptr, method_len, url_ptr, url_len, body_ptr, body_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_http_serve(port: i32, body_ptr: i32, body_len: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_http_serve(port, body_ptr, body_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_sockets_connect(host_ptr: i32, host_len: i32, port: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_socket_connect(host_ptr, host_len, port), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_sockets_read(socket: i32, max_len: i32, resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_socket_read(socket, max_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_sockets_write(socket: i32, ptr: i32, len: i32, resp_ptr: i32) -> i32 {
    let buffer = unsafe { buffer_from_memory(ptr, len) };
    let result = unsafe { host_socket_write(socket, buffer) };
    unsafe { host_buffer_close(buffer) };
    unsafe { scalar_or_error(result, resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_sockets_listen(host_ptr: i32, host_len: i32, port: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_socket_listen(host_ptr, host_len, port), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_sockets_accept(listener: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_socket_accept(listener), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_open_at(_fd: i32, _dirflags: i32, path_ptr: i32, path_len: i32, _oflags: i32, _rights_base: i64, _rights_inheriting: i64, _fdflags: i32, out_fd: i32) -> i32 {
    unsafe {
        FS_PATH_PTR = path_ptr;
        FS_PATH_LEN = path_len;
        store_i32(out_fd, 4);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_read(_fd: i32, iovs: i32, _iovs_len: i32, nread: i32) -> i32 {
    let (path_ptr, path_len) = unsafe { (FS_PATH_PTR, FS_PATH_LEN) };
    let handle = unsafe { host_fs_read_file(path_ptr, path_len) };
    if handle <= 0 {
        if handle < 0 {
            unsafe { host_buffer_close(-handle) };
        }
        return 1;
    }
    let dst = unsafe { load_i32(iovs) };
    let cap = unsafe { load_i32(iovs + 4) };
    let available = unsafe { host_buffer_len(handle) };
    let count = if available < cap { available } else { cap };
    let mut i = 0;
    while i < count {
        unsafe { store_u8(dst + i, host_buffer_byte(handle, i)) };
        i += 1;
    }
    unsafe {
        host_buffer_close(handle);
        store_i32(nread, count);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_write(_fd: i32, iovs: i32, _iovs_len: i32, nwritten: i32) -> i32 {
    let ptr = unsafe { load_i32(iovs) };
    let len = unsafe { load_i32(iovs + 4) };
    let buffer = unsafe { buffer_from_memory(ptr, len) };
    let (path_ptr, path_len) = unsafe { (FS_PATH_PTR, FS_PATH_LEN) };
    let result = unsafe { host_fs_write_bytes(path_ptr, path_len, buffer) };
    unsafe { host_buffer_close(buffer) };
    if result < 0 {
        unsafe { host_buffer_close(-result) };
        return 1;
    }
    unsafe { store_i32(nwritten, len) };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn runtime_fs_close(_fd: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_read_dir(path_ptr: i32, path_len: i32, resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_fs_read_dir(path_ptr, path_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_metadata(path_ptr: i32, path_len: i32, resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_fs_metadata(path_ptr, path_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_remove_file(path_ptr: i32, path_len: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_fs_remove_file(path_ptr, path_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_fs_create_dir_all(path_ptr: i32, path_len: i32, resp_ptr: i32) -> i32 {
    unsafe { scalar_or_error(host_fs_create_dir_all(path_ptr, path_len), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_env_vars(resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_env_vars(), resp_ptr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn runtime_env_current_dir(resp_ptr: i32) -> i32 {
    unsafe { copy_handle_payload(host_env_current_dir(), resp_ptr) }
}
