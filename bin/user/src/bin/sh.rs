#![no_std]
#![no_main]

use alloc::rc::Rc;
use alloc::boxed::Box;
use core::cell::RefCell;
use core::mem::size_of;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use user::*;

const MAXARGS: usize = 10;

enum CMD {
    ExecCMD(Rc<RefCell<ExecCmd>>),
    RedirCMD(Box<RedirCMD>),
    PipeCMD(Box<PipeCMD>),
    ListCMD(Box<ListCMD>),
    BackCMD(Box<BackCMD>),
}

struct ExecCmd {
    pub argv: [*const u8; MAXARGS],
    pub eargv: [*mut u8; MAXARGS],
}

struct RedirCMD {
    pub cmd: CMD,
    pub file: *const u8,
    pub efile: *mut u8,
    pub mode: usize,
    pub fd: usize,
}

struct PipeCMD {
    pub left: CMD,
    pub right: CMD,
}

struct ListCMD {
    pub left: CMD,
    pub right: CMD,
}

struct BackCMD {
    pub cmd: CMD,
}

fn runcmd(cmd: &CMD) {
    let mut p = [0usize; 2];

    match cmd {
        CMD::ExecCMD(ecmd) => {
            if ecmd.borrow().argv[0] == 0 as *const u8 {
                exit(1);
            }
            let name = unsafe {
                from_utf8_unchecked(from_raw_parts(
                    ecmd.borrow().argv[0],
                    strlen(ecmd.borrow().argv[0]),
                ))
            };
            exec(name, &ecmd.borrow().argv);
            fprintln!(1, "exec {} failed", name);
        }
        CMD::RedirCMD(rcmd) => {
            close(rcmd.fd);
            let name = unsafe {
                from_utf8_unchecked(from_raw_parts(
                    rcmd.file,
                    strlen(rcmd.file),
                ))
            };
            if open(name, rcmd.mode) < 0 {
                fprintln!(1, "open {} failed", name);
                exit(1);
            }
            runcmd(&rcmd.cmd);
        }
        CMD::ListCMD(lcmd) => {
            if fork1() == 0 {
                runcmd(&lcmd.left);
            }
            wait(0 as *mut usize);
            runcmd(&lcmd.right);
        }
        CMD::PipeCMD(pcmd) => {
            if pipe(&mut p) < 0 {
                panic!("pipe");
            }
            if fork1() == 0 {
                close(1);
                dup(p[1]);
                close(p[0]);
                close(p[1]);
                runcmd(&pcmd.left);
            }
            if fork1() == 0 {
                close(0);
                dup(p[0]);
                close(p[0]);
                close(p[1]);
                runcmd(&pcmd.right);
            }
            close(p[0]);
            close(p[1]);
            wait(0 as *mut usize);
            wait(0 as *mut usize);
        }
        CMD::BackCMD(bcmd) => {
            if fork1() == 0 {
                runcmd(&bcmd.cmd);
            }
        }
    }
    exit(0);
}

fn getcmd(buf: &mut [u8], nbuf: usize) -> i32 {
    fprint!(1, "$ ");
    buf.fill(0);
    gets(buf, nbuf);
    if buf[0] == 0 {
        // EOF
        -1
    } else {
        0
    }
}

#[no_mangle]
pub fn main(_args: Vec<&str>) {
    let mut buf = [0u8; 100];
    let mut fd = open("console", OPEN_READ_WRITE);
    while fd >= 0 {
        if fd >= 3 {
            close(fd as usize);
            break;
        }
        fd = open("console", OPEN_READ_WRITE);
    }

    // Read and run input commands.
    while getcmd(&mut buf, size_of::<[u8; 100]>()) >= 0 {
        if buf[0] == b'c' && buf[1] == b'd' && buf[2] == b' ' {
            // Chdir must be called by the parent, not the child.
            buf[strlen(buf.as_ptr()) - 1] = 0; // chop \n
            unsafe {
                let p = buf.as_ptr().add(3);
                if chdir(p) < 0 {
                    fprintln!(
                        1,
                        "cannot cd {}",
                        from_utf8_unchecked(from_raw_parts(p, strlen(p)))
                    );
                }
            }
            continue;
        }
        if fork1() == 0 {
            unsafe {
                runcmd(&parsecmd(buf.as_mut_ptr()));
            }
        }
        wait(0 as *mut usize);
    }
}

fn fork1() -> isize {
    let pid = fork();
    if pid == -1 {
        panic!("fork");
    }
    pid
}

fn execcmd() -> CMD {
    CMD::ExecCMD(Rc::new(RefCell::new(ExecCmd {
        argv: [0 as *const u8; MAXARGS],
        eargv: [0 as *mut u8; MAXARGS],
    })))
}

fn redircmd(subcmd: CMD, file: *const u8, efile: *mut u8, mode: usize, fd: usize) -> CMD {
    CMD::RedirCMD(Box::new(RedirCMD {
        cmd: subcmd,
        file,
        efile,
        mode,
        fd,
    }))
}

fn pipecmd(left: CMD, right: CMD) -> CMD {
    CMD::PipeCMD(Box::new(PipeCMD { left, right }))
}

fn listcmd(left: CMD, right: CMD) -> CMD {
    CMD::ListCMD(Box::new(ListCMD { left, right }))
}

fn backcmd(subcmd: CMD) -> CMD {
    CMD::BackCMD(Box::new(BackCMD { cmd: subcmd }))
}

const WHITESPACE: &str = " \t\r\n";
const SYMBOLS: &str = "<|>&;()";

unsafe fn gettoken(
    ps: &mut *mut u8,
    es: *const u8,
    q: Option<&mut *const u8>,
    eq: Option<&mut *mut u8>,
) -> u8 {
    let mut s: *const u8 = *ps;
    while s < es && strchr(WHITESPACE, *s) {
        s = s.add(1);
    }
    if let Some(x) = q {
        *x = s;
    }
    let mut ret = *s;

    match *s {
        0 => (),
        b'|' | b'(' | b')' | b';' | b'&' | b'<' => {
            s = s.add(1);
        }
        b'>' => {
            s = s.add(1);
            if *s == b'>' {
                ret = b'+';
                s = s.add(1);
            }
        }
        _ => {
            ret = b'a';
            while s < es && !strchr(WHITESPACE, *s) && !strchr(SYMBOLS, *s) {
                s = s.add(1);
            }
        }
    }
    if let Some(x) = eq {
        *x = s as *mut u8;
    }

    while s < es && strchr(WHITESPACE, *s) {
        s = s.add(1);
    }

    *ps = s as *mut u8;

    ret
}

unsafe fn peek(ps: &mut *mut u8, es: *const u8, toks: &str) -> bool {
    let mut s: *const u8 = *ps;

    while s < es && strchr(WHITESPACE, *s) {
        s = s.add(1);
    }

    *ps = s as *mut u8;

    *s != 0 && strchr(toks, *s)
}

unsafe fn parsecmd(mut s: *mut u8) -> CMD {
    let es = s.add(strlen(s));
    let mut cmd = parseline(&mut s, es);
    peek(&mut s, es, "");
    if s != es {
        fprintln!(
            1,
            "leftovers: {}",
            from_utf8_unchecked(from_raw_parts(s, strlen(s)))
        );
        panic!("syntax");
    }
    nulterminate(&mut cmd);
    cmd
}

unsafe fn parseline(ps: &mut *mut u8, es: *const u8) -> CMD {
    let mut cmd = parsepipe(ps, es);
    while peek(ps, es, "&") {
        gettoken(ps, es, None, None);
        cmd = backcmd(cmd);
    }
    if peek(ps, es, ";") {
        gettoken(ps, es, None, None);
        cmd = listcmd(cmd, parseline(ps, es));
    }
    cmd
}

unsafe fn parsepipe(ps: &mut *mut u8, es: *const u8) -> CMD {
    let mut cmd = parseexec(ps, es);
    if peek(ps, es, "|") {
        gettoken(ps, es, None, None);
        cmd = pipecmd(cmd, parsepipe(ps, es));
    }
    cmd
}

unsafe fn parseredirs(mut cmd: CMD, ps: &mut *mut u8, es: *const u8) -> CMD {
    let mut q = 0 as *const u8;
    let mut eq = 0 as *mut u8;

    while peek(ps, es, "<>") {
        let tok = gettoken(ps, es, None, None);
        if gettoken(ps, es, Some(&mut q), Some(&mut eq)) != b'a' {
            panic!("missing file for redirection");
        }
        match tok {
            b'<' => {
                cmd = redircmd(cmd, q, eq, OPEN_READ_ONLY, 0);
            }
            b'>' => {
                cmd = redircmd(cmd, q, eq, OPEN_WRITE_ONLY | OPEN_CREATE | OPEN_TRUNC, 1);
            }
            b'+' => {
                cmd = redircmd(cmd, q, eq, OPEN_WRITE_ONLY | OPEN_CREATE, 1);
            }
            _ => (),
        }
    }
    cmd
}

unsafe fn parseblock(ps: &mut *mut u8, es: *const u8) -> CMD {
    if !peek(ps, es, "(") {
        panic!("parseblock");
    }
    gettoken(ps, es, None, None);
    let mut cmd = parseline(ps, es);
    if !peek(ps, es, ")") {
        panic!("syntax - missing )");
    }
    gettoken(ps, es, None, None);
    cmd = parseredirs(cmd, ps, es);
    cmd
}

unsafe fn parseexec(ps: &mut *mut u8, es: *const u8) -> CMD {
    let mut q = 0 as *const u8;
    let mut eq = 0 as *mut u8;

    if peek(ps, es, "(") {
        parseblock(ps, es);
    }

    let mut ret = execcmd();
    let cmd: Rc<RefCell<ExecCmd>>;
    if let CMD::ExecCMD(x) = &ret {
        cmd = x.clone();
    } else {
        panic!();
    }

    let mut argc = 0;
    ret = parseredirs(ret, ps, es);
    while !peek(ps, es, "|)&;") {
        let tok = gettoken(ps, es, Some(&mut q), Some(&mut eq));
        if tok == 0 {
            break;
        }
        if tok != b'a' {
            panic!("syntax");
        }
        cmd.borrow_mut().argv[argc] = q;
        cmd.borrow_mut().eargv[argc] = eq;
        argc += 1;
        if argc >= MAXARGS {
            panic!("too many args");
        }
        ret = parseredirs(ret, ps, es);
    }
    cmd.borrow_mut().argv[argc] = 0 as *const u8;
    cmd.borrow_mut().eargv[argc] = 0 as *mut u8;

    ret
}

fn nulterminate(cmd: &mut CMD) {
    match cmd {
        CMD::ExecCMD(ecmd) => {
            for i in 0..MAXARGS {
                if ecmd.borrow().argv[i] != 0 as *const u8 {
                    unsafe {
                        *ecmd.borrow_mut().eargv[i] = 0;
                    }
                }
            }
        }
        CMD::RedirCMD(rcmd) => {
            nulterminate(&mut rcmd.cmd);
            unsafe {
                *rcmd.efile = 0;
            }
        }
        CMD::PipeCMD(pcmd) => {
            nulterminate(&mut pcmd.left);
            nulterminate(&mut pcmd.right);
        }
        CMD::ListCMD(lcmd) => {
            nulterminate(&mut lcmd.left);
            nulterminate(&mut lcmd.right);
        }
        CMD::BackCMD(bcmd) => {
            nulterminate(&mut bcmd.cmd);
        }
    }
}
