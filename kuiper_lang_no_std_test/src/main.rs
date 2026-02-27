#![no_std]
#![no_main]

extern crate alloc;

use kuiper_lang::{alloc::string::ToString, compile_expression};
use linux_syscall::syscall;
use talc::{ClaimOnOom, Span, Talc, Talck};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

static mut ARENA: [u8; 10_000] = [0; 10_000];

#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // if we're in a hosted environment, the Rust runtime may allocate before
    // main() is called, so we need to initialize the arena automatically
    ClaimOnOom::new(Span::from_array(core::ptr::addr_of!(ARENA).cast_mut()))
})
.lock();

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Align stack pointer to 16 bytes
    // Without this you get weirdo segfaults since some of the lalrpop code happens
    // to use SSE instructions which require 16-byte alignment
    unsafe {
        core::arch::asm!(
            "and rsp, 0xFFFFFFFFFFFFFFF0",
            options(nostack, nomem, preserves_flags)
        );
    }
    let expr = compile_expression("[1, 2, 3].map(x => x + 1)", &[]).unwrap();
    let res = expr.run([]).unwrap();
    let r = alloc::format!("{}\n\0", res.to_string());
    // Manually write the result to stdout
    unsafe {
        let _ = syscall!(linux_syscall::SYS_write, 1, r.as_ptr(), r.len());
    }
    loop {}
}
