static EXEC: i32 =1;
static REDIR: i32 =2;
static PIPE: i32 =3;
static LIST: i32 =4;
static BACK: i32 =5;
static MAXARGS: i32=10;

struct cmd{
    Type:i32,
    ptr: *mut_,
}
struct execcmd{
    Type: i32,
    argv: [String; MAXARGS as usize],
    eargv:[String; MAXARGS as usize],
}
struct redircmd{
    Type: i32,
    cmd: cmd,
    file: String,
    efile: String,
    mode: i32,
    fd: i32,
}
struct pipecmd{
    Type: i32,
    left: cmd,
    right: cmd,
}
struct listcmd{
    Type: i32,
    left: cmd,
    right: cmd,
}
struct backcmd{
    Type: i32,
    cmd: cmd,
}
// pub trait From<T> {
//     fn from(T) -> Self;
// }
// impl From<cmd> for execcmd{
//     fn from(cmd:cmd) -> Self{
//         execcmd{
//             Type:cmd.Type,
//             argv: [' ';MAXARGS],
//             eargv: [' ';MAXARGS],
//         }
//     }
// }
// impl From<cmd> for redircmd{
//     fn from(cmd:cmd) -> Self{
//         redircmd{
//             Type:cmd.Type,
//             cmd: cmd,
//             file: "",
//             efile: "",
//             mode:0,
//             fd:0,
//         }
//     }
// }
pub unsafe fn runcmd(cmd : cmd){
    let mut p = [i32;2];
    if cmd==0{
        exit(); //todo
    }
    match cmd.Type{
        EXEC =>{
            let mut ecmd= unsafe{*cmd.ptr};
            if ecmd.argv[0]==0{
                exit();
            }
            exec(ecmd.argv[0],ecmd.argv);//todo
            println!("exec {} failed\n", ecmd.argv[0]);
        },
        REDIR =>{
            let mut rcmd= cmd as redircmd;
            close(rcmd.fd);// todo
            if open(rcmd.file,rcmd.mode)<0{
                fprintf(2,"open %s failed\n", rcmd.file);//todo
                exit();
            }
            runcmd(runcmd.cmd);
        }
        LIST => {
            let mut lcmd :listcmd= unsafe{*cmd.ptr};
            if fork1()==0 {
                runcmd(lcmd.left);
            }
            wait();
            runcmd(lcmd.right);
        }
        PIPE => {
            let mut pcmd= unsafe{*cmd.ptr};
            if pipe(p) < 0 {
                panic!("pipe");
            }
            if fork1() == 0 {
                close(1);
                dup(p[1]);
                close(p[0]);
                close(p[1]);
                runcmd(pcmd.left);
            }
            if fork1() == 0{
                close(0);
                dup(p[0]);
                close(p[0]);
                close(p[1]);
                runcmd(pcmd.right);
              }
              close(p[0]);
              close(p[1]);
              wait();
              wait();
        }
        BACK => {
            let mut bcmd= unsafe{*cmd.ptr};
            if fork1() == 0{
                runcmd(backcmd.cmd);
            }
        }
        _ => {}
    }
    exit();
}

pub unsafe fn getcmd(buf: String, nbuf: i32) -> i32{
    fprintf(2, "$ ");
    memset(buf, 0, nbuf);
    gets(buf, nbuf);
    if buf[0] == 0{
        -1
    } 
    0
}

unsafe fn main(){
    let mut  buf =String::new();
    let mut fd: i32;
    fd = open("console", O_RDWR);
  while fd  >= 0{
        if fd >= 3 {
          close(fd);
          break;
        }
      fd = open("console", O_RDWR);
  }

  // Read and run input commands.
  while getcmd(buf, sizeof(buf)) >= 0 {
    if buf[0] == 'c' && buf[1] == 'd' && buf[2] == ' ' {
      // Chdir must be called by the parent, not the child.
      buf[strlen(buf)-1] = 0;  // chop \n
      if chdir(buf+3) < 0 {
          fprintf(2, "cannot cd %s\n", buf+3);
      }
      continue;
    }
    if fork1() == 0{
         runcmd(parsecmd(buf));
    }
    wait();
  }
  exit();
}

pub unsafe fn fork1() ->i32{
    let mut pid = fork();
  if pid == -1{
      panic!("fork");
  }
   pid
}
impl execcmd{
    fn execcmd() -> cmd{
        let x =execcmd{
            Type:EXEC,
            argv:[" ";MAXARGS],
            eargv:[" ";MAXARGS],
        };
        cmd{
            Type:EXEC,
            ptr:&x,
        }
    }
}
impl redircmd{
    fn redircmd(subcmd:cmd,file:&str,efile:&str,mode: i32, fd: i32) -> cmd{
        let x =redircmd{
            Type:REDIR,
            cmd:subcmd,
            file,
            efile,
            mode,
            fd
        };
        cmd{
            Type:REDIR,
            ptr:&x,
        }
    }
}
impl pipecmd{
    fn pipecmd(left: cmd,right:cmd) -> cmd{
        let x =pipecmd{
            Type:PIPE,
            left,
            right
        };
        cmd{
            Type:PIPE,
            ptr:&x,
        }
    }
}
impl listcmd{
    fn listcmd(left: cmd,right:cmd) -> cmd{
        let x =listcmd{
            Type:LIST,
            left,
            right
        };
        cmd{
            Type:LIST,
            ptr:&x,
        }
    }
}
impl backcmd{
    fn backcmd(subcmd:cmd) -> cmd{
        let x =backcmd{
            Type:BACK,
            cmd:backcmd
        };
        cmd{
            Type:BACK,
            ptr:&x,
        }
    }
}

static whitespace:&str =" \t\r\n";
static symbols:&str ="<|>&;()";


pub unsafe fn gettoken(mut ps: &String, es:&String,mut  q: &String,mut  eq: &String) -> i32{
    let mut s=ps.trim();
    q=s;
    let mut n:i32=0;
    let mut v: Vec<char> = s.chars().collect();
    let mut ret : i32=v[0] as i32;
    match v[n]{
        0 => {},
        '|' => {},
        ')' => {},
        '(' => {},
        ';' => {},
        '&' => {},
        '<' => {n=n+1;},
        '>' => {
            n=n+1;
            if v[n] =='>'{
                ret ='+' as i32;
                n=n+1;
            }
        },
        _ => {
            ret= 'a' as i32;
            for i in ps.chars(){
                if !(i==' '|| i=='\t' || i=='\r' || i=='\n' )
                &&!(i=='<'||i=='|'||i=='>'||i=='&'||i==';'||i=='('){
                    n=n+1;
                }
                else{
                    break;
                }
            }
        }
    };
    s=String::from(&s[n..]);
    eq=s;
    ps=s.trim();
    ret

}
pub unsafe fn peek(mut ps:&String,mut es:&String,mut toks:&String) -> bool{
    let mut n=0;
    for i in ps.chars(){
        if i==' '|| i=='\t' || i=='\r' || i=='\n'{
            n=n+1;
        }
        else{
            break;
        }
    }
    ps=String::from(&ps[n..]);
    let mut v: Vec<char> = ps.chars().collect();
    for i in pss.chars(){
       for j in toks.chars(){
           if i==j{
               true
           }
       }
    }
    false
}
pub unsafe fn parsecmd(mut s:&String) ->cmd{
    let mut es:String=String::from("");
    let mut t:Vec<String>=s.split(" " as u8).collect();
    let mut cmd: cmd=parseline(t,es);
    peek(t,es,"");
    if s!=es {
        fprintf(2, "leftovers: %s\n", s);
        panic("syntax");
    }
    nulterminate(&cmd);
    cmd
}
pub unsafe fn parseline(mut ps:&String,mut es:&String) ->cmd{
    let mut cmd:cmd=parsepipe(ps,es);
    while peek(ps,es,"&") {
        gettoken(ps,es,"","");
        cmd=backcmd(cmd);
    }
    if peek(ps,es,";") {
        gettoken(ps,es,"","");
        cmd=listcmd(cmd,parseline(ps,es));
    }
    cmd
}
pub unsafe fn parsepipe(mut ps:&String,mut es:&String) ->cmd{
    let mut cmd:cmd =parseexec(ps,es);
    if peek(ps,es,"|"){
        gettoken(ps,es,"","");
        cmd=pipecmd(cmd,parsepipe(ps,es));
    }
    cmd;
}
pub unsafe fn parseredirs(mut cmd: cmd,mut ps:&String,mut es: &String) ->cmd{
    let mut tok=0;
    let mut q=String::new();
    let mut eq=String::new();
    while peek(ps,es,"<>"){
        tok=gettoken(ps,es,"","");
        if gettoken(ps,es,q,eq) !='a' as i32{panic("missing file for redirection");}
        match tok as char{
            '<' => {cmd=redircmd(cmd,q,eq,O_RDONLY,0);},
            '>' => {cmd=redircmd(cmd,q,eq,O_WRONLY|O_CREATE,1);},
            '+' => {cmd = redircmd(cmd, q, eq, O_WRONLY|O_CREATE, 1);},
            _ =>{},
        }
    }
    CMD
}
pub unsafe fn parseblock(mut ps:&String,mut es:&String) ->cmd{
    if !peek(ps,es,"("){
        panic("parseblock");
    }
    gettoken(ps,es,"","");
    let mut cmd=parseline(ps,es);
    if !peek(ps,es,")") {
        panic("syntax - missing )");
    }
    gettoken(ps, es, "", "");
    cmd = parseredirs(cmd, ps, es);
    cmd
}
pub unsafe fn parseexec(mut s:&String,mut es:&String) ->cmd{
    if peek(ps,es,"(") {
        parseblock(ps,es)
    }
    let mut ret=execcmd();
    let mut cmd= ret.ptr;
    let mut argc=0;
    ret=parseredirs(ret,ps,es);
    while !peek(ps,es,"|)&;"){
        tok=gettoken(ps, es, &q, &eq);
        if tok==0{
            break;
        }
        if tok!='a'{
            panic("syntax");
        }
        cmd.argv[argc]=q;
        cmd.eargv[argc]=eq;
        argc=argc+1;
        if argc>= MAXARGS{
            panic("too many args");
        }
        ret=parseredirs(ret, ps, es);
    }
    cmd.argv[argc]=0;
    cmd.eargv[argc]=0;
    ret
}
pub unsafe fn nulterminate(cmd: &cmd) ->&cmd{

    match cmd.Type{
        EXEC =>{
            let mut ecmd: execcmd=unsafe{*cmd.ptr};
            for i in 0..MAXARGS{
                if ecmd.argv[i]!=""
                    *ecmd.eargv[i]=0;
            }
        },
        REDIR =>{
            let mut rcmd:redircmd=unsafe{*cmd.ptr};
            nulterminate(&rcmd.cmd);
            *rcmd.efile=0;
        }
        PIPE =>{
            let mut pcmd:pipecmd=unsafe{*cmd.ptr};
            nulterminate(&pcmd.left);
            nulterminate(&pcmd.right);
        }
        LIST =>{
            let mut lcmd:listcmd=unsafe{*cmd.ptr};
            nulterminate(&lcmd.left);
            nulterminate(&lcmd.right);
        }
        BACK =>{
            let mut bcmd:backcmd=unsafe{*cmd.ptr};
            nulterminate(&bcmd.cmd);
        }
        _ => {}
    }
    cmd
}
