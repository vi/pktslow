use tun_tap::{Iface,Mode};

use crossbeam_utils::thread::scope;
use crossbeam_channel::{unbounded,Sender,Receiver};
use std::time::{Duration,Instant};

use argh::FromArgs;

//use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU32,Ordering,AtomicU8};

/// simple program that creates a veth-like pair of TUN interfaces
/// and allows to selectively delay some packets
/// 
/// Other options are specified as interactive stdin commands
#[derive(FromArgs)]
struct Opt {
    /// name of the first tunnel device
    #[argh(positional)]
    tun1n : String,

    /// name of the second tunnel device
    #[argh(positional)]
    tun2n : String,
}

/// interactive options
#[derive(FromArgs)]
struct StdinOpt {
    #[argh(subcommand)]
    cmd: StdinCmd,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum StdinCmd {
    Quit(Quit),
    AdjustDelay(AdjustDelay),
    Monitor(Monitor),
    Stats(Stats),
    SetupMask(SetupMask),
}
/// Exit from process
#[derive(FromArgs)]
#[argh(subcommand)]
#[argh(name="quit")]
struct Quit { }

/// Adjust the delay
#[derive(FromArgs)]
#[argh(subcommand)]
#[argh(name="delay")]
struct AdjustDelay {
    /// number of milliseconds matching packets delayed by
    #[argh(positional)]
    delay_ms : u32
}


/// Print packets content to stdout
#[derive(FromArgs)]
#[argh(subcommand)]
#[argh(name="mon")]
struct Monitor {
    #[argh(positional)]
    offset: u32,

    #[argh(positional,default = "8")]
    len: u32,

    /// stop monitoring after this number of printed lines
    #[argh(option, short='m', default="10")]
    max_occurs : u32,
}

/// Show statistics
#[derive(FromArgs)]
#[argh(subcommand)]
#[argh(name="stats")]
struct Stats { }

/// Setup matcher that would decide whether to delay packets
#[derive(FromArgs)]
#[argh(subcommand)]
#[argh(name="setup")]
struct SetupMask {
    #[argh(positional)]
    offset: u32,

    #[argh(positional)]
    value: u8,

    #[argh(positional,default = "255")]
    mask: u8,
}



static DELAY : AtomicU32 = AtomicU32::new(50);

static MONITOR: AtomicU32 = AtomicU32::new(0);
static MONITOR_OFF: AtomicU32 = AtomicU32::new(0);
static MONITOR_LEN: AtomicU32 = AtomicU32::new(0);

static DELAYED: AtomicU32 = AtomicU32::new(0);
static NONDELAYED: AtomicU32 = AtomicU32::new(0);

static DL_OFF: AtomicU32 = AtomicU32::new(0);
static DL_MASK: AtomicU8 = AtomicU8::new(0xFF);
static DL_VAL: AtomicU8 = AtomicU8::new(0);


fn copy(ifr: &Iface, ifw: &Iface, del:Sender<(Instant, Vec<u8>)>) {
    let mut buf = [0u8; 4096];
    
    while let Ok(l) = ifr.recv(&mut buf[..]) {
        let buf : &[u8] = & buf[0..l];

        let (mon,monoff,monlen) = (
            MONITOR.load(Ordering::Acquire),
            MONITOR_OFF.load(Ordering::Relaxed),
            MONITOR_LEN.load(Ordering::Relaxed),
        );

        if mon > 0 && monoff <= l as u32 {
            print!("PKT l={:<4} ", l);
            for x in (monoff)..(monoff+monlen) {
                let x = x as usize;
                if x < buf.len() {
                    if x % 4 == 0 {
                        print!(" ");
                    }
                    if x % 16 == 0 {
                        print!("  ");
                    }
                    print!("{:02X}", buf[x]);
                }
            }
            println!();
            MONITOR.store(mon-1, Ordering::Release);
        }

        let mut mood = false;
        
        let (doff,dmask,dval) = (
            DL_OFF.load(Ordering::Acquire),
            DL_MASK.load(Ordering::Relaxed),
            DL_VAL.load(Ordering::Relaxed),
        );

        if doff < l as u32 {
            if (buf[doff as usize] & dmask) == dval {
                mood = true;
            }
        }

        if !mood {
            NONDELAYED.fetch_add(1, Ordering::Relaxed);
            let _ = ifw.send(buf);
        } else {
            DELAYED.fetch_add(1, Ordering::Relaxed);
            let _ = del.send((Instant::now() + Duration::from_millis(DELAY.load(Ordering::Relaxed) as u64), buf.to_vec()));
        }
    }
}

fn delayline(if_: &Iface, r: Receiver<(Instant, Vec<u8>)>) {
    while let Ok((t, b)) = r.recv() {
        std::thread::sleep(t.saturating_duration_since(Instant::now()));
        let _ = if_.send(&b[..]);
    }
}

fn adjuster() {
    let si = std::io::stdin();
    let si = si.lock();
    let si = std::io::BufReader::new(si);
    use std::io::BufRead;
    println!("Type --help to see stdin commands");
    for l in si.lines() {
        if let Ok(l) = l {
            let v : Vec<&str> = l.split_whitespace().collect();
            match StdinOpt::from_args(
                    &vec![][..],
                    &v[..]
                ) {
                Ok(opt) => {
                    match opt.cmd {
                        StdinCmd::Quit(_) => std::process::exit(0),
                        StdinCmd::AdjustDelay(AdjustDelay{delay_ms}) => {
                            DELAY.store(delay_ms, Ordering::Relaxed);
                        }
                        StdinCmd::Monitor(Monitor{
                            offset, len, max_occurs
                        }) => {
                            MONITOR_OFF.store(offset, Ordering::Relaxed);
                            MONITOR_LEN.store(len, Ordering::Relaxed);
                            MONITOR.store(max_occurs, Ordering::Release);
                        }
                        StdinCmd::Stats(_) => {
                            println!(
                                "delayed: {},  nondelayed: {}",
                                DELAYED.load(Ordering::Acquire),
                                NONDELAYED.load(Ordering::Relaxed),
                            );
                        }
                        StdinCmd::SetupMask(SetupMask {
                            offset,
                            value,
                            mask
                        }) => {
                            DL_OFF.store(offset, Ordering::Relaxed);
                            DL_MASK.store(mask, Ordering::Relaxed);
                            DL_VAL.store(value, Ordering::Release);
                        }
                    }
                }
                Err(early) => {
                    println!("{}", early.output);
                }  
            } 
        } else {
            eprintln!("Failed reading a line");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opt : Opt = argh::from_env();

    let if1 = Iface:: without_packet_info(&opt.tun1n, Mode::Tun)?;
    let if2 = Iface:: without_packet_info(&opt.tun2n, Mode::Tun)?;

    let _ = scope(|s|  {
        let (del1s, del1r) = unbounded();
        let (del2s, del2r)  = unbounded();

        let _h = s.spawn(|_| {
            copy(&if1, &if2, del1s);
        });
        let _h = s.spawn(|_| {
            delayline(&if2, del1r);
        });
        

        let _h = s.spawn(|_| {
            copy(&if2, &if1, del2s);
        });
        let _h = s.spawn(|_| {
            delayline(&if1, del2r);
        });

        let _h = s.spawn(|_| {
            adjuster()
        });
    });


    Ok(())
}
