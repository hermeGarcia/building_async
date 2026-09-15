use std::arch::{asm, naked_asm};

const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;
const MAX_THREADS: usize = 4;
static mut RUNTIME: usize = 0;

pub struct Rustime {
    threads: Vec<Thread>,
    current: usize,
}

#[derive(PartialEq, Eq, Debug)]
enum State {
    Available,
    Running,
    Ready,
}

struct Thread {
    stack: Vec<u8>,
    ctx: ThreadContext,
    state: State,
}

#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    // Stack pointer
    sp: u64,
    // Callee-saved registers
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    // Frame pointer
    fp: u64,
    // Link register
    lr: u64,
}

impl Thread {
    fn new() -> Self {
        Thread {
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Available,
        }
    }
}

impl Rustime {
    pub fn new() -> Self {
        let base_thread = Thread {
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
            state: State::Running,
        };

        let mut threads = vec![base_thread];
        let mut available_threads: Vec<Thread> = (1..MAX_THREADS).map(|_| Thread::new()).collect();
        threads.append(&mut available_threads);

        Rustime {
            threads,
            current: 0,
        }
    }

    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Rustime = self;
            RUNTIME = r_ptr as usize;
        }
    }

    pub fn run(&mut self) -> ! {
        while self.t_yield() {}
        std::process::exit(0);
    }

    fn t_return(&mut self) {
        if self.current != 0 {
            self.threads[self.current].state = State::Available;
            self.t_yield();
        }
    }

    #[inline(never)]
    fn t_yield(&mut self) -> bool {
        let mut pos = self.current;
        while self.threads[pos].state != State::Ready {
            pos += 1;
            if pos == self.threads.len() {
                pos = 0;
            }
            if pos == self.current {
                return false;
            }
        }

        if self.threads[self.current].state != State::Available {
            self.threads[self.current].state = State::Ready;
        }

        self.threads[pos].state = State::Running;
        let old_pos = self.current;
        self.current = pos;

        unsafe {
            let old: *mut ThreadContext = &mut self.threads[old_pos].ctx;
            let new: *const ThreadContext = &self.threads[pos].ctx;
            asm!("bl switch", in("x0") old, in("x1") new, clobber_abi("C"));
        }
        self.threads.len() > 0
    }

    pub fn spawn(&mut self, f: fn()) {
        let available = self
            .threads
            .iter_mut()
            .find(|t| t.state == State::Available)
            .expect("no available thread.");

        let size = available.stack.len();

        unsafe {
            let s_ptr = available.stack.as_mut_ptr().offset(size as isize);
            let s_ptr = (s_ptr as usize & !15) as *mut u8;
            available.ctx.sp = s_ptr as u64;
            available.ctx.x19 = f as u64;
            available.ctx.lr = call_fn as u64;
        }
        available.state = State::Ready;
    }
}

#[cfg_attr(target_os = "macos", unsafe(export_name = "\x01prologue"))]
fn prologue() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Rustime;
        (*rt_ptr).t_return();
    };
}

// Trampoline that calls the fiber's entry point (stashed in x19 by `spawn`)
// and then tail-branches into `prologue` when it returns (for AArch64).
#[unsafe(naked)]
unsafe extern "C" fn call_fn() {
    naked_asm!("mov x0, x19", "blr x0", "b prologue")
}

pub fn yield_thread() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Rustime;
        (*rt_ptr).t_yield();
    };
}

#[unsafe(naked)]
#[cfg_attr(target_os = "macos", unsafe(export_name = "\x01switch"))]
unsafe extern "C" fn switch() {
    naked_asm!(
        "mov x9, sp",
        "str x9, [x0, #0x00]",
        "str x19, [x0, #0x08]",
        "str x20, [x0, #0x10]",
        "str x21, [x0, #0x18]",
        "str x22, [x0, #0x20]",
        "str x23, [x0, #0x28]",
        "str x24, [x0, #0x30]",
        "str x25, [x0, #0x38]",
        "str x26, [x0, #0x40]",
        "str x27, [x0, #0x48]",
        "str x28, [x0, #0x50]",
        "str x29, [x0, #0x58]",
        "str x30, [x0, #0x60]",
        "ldr x9, [x1, #0x00]",
        "mov sp, x9",
        "ldr x19, [x1, #0x08]",
        "ldr x20, [x1, #0x10]",
        "ldr x21, [x1, #0x18]",
        "ldr x22, [x1, #0x20]",
        "ldr x23, [x1, #0x28]",
        "ldr x24, [x1, #0x30]",
        "ldr x25, [x1, #0x38]",
        "ldr x26, [x1, #0x40]",
        "ldr x27, [x1, #0x48]",
        "ldr x28, [x1, #0x50]",
        "ldr x29, [x1, #0x58]",
        "ldr x30, [x1, #0x60]",
        "ret"
    );
}

fn main() {
    let mut runtime = Rustime::new();
    runtime.init();

    runtime.spawn(|| {
        println!("THREAD 1 STARTING");
        for _ in 0..10 {
            yield_thread();
        }
        println!("THREAD 1 FINISHED");
    });

    runtime.spawn(|| {
        println!("THREAD 2 STARTING");
        for _ in 0..15 {
            yield_thread();
        }
        println!("THREAD 2 FINISHED");
    });
    runtime.run();
}
