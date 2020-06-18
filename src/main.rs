use tun_tap::{Iface,Mode};

use crossbeam_utils::thread::scope;
use crossbeam_channel::{unbounded,Sender,Receiver};
use std::time::{Duration,Instant};

fn copy(ifr: &Iface, ifw: &Iface, del:Sender<(Instant, Vec<u8>)>) {
    let mut buf = [0u8; 4096];
    let mut mood = false;
    while let Ok(l) = ifr.recv(&mut buf[..]) {
        let buf : &[u8] = & buf[0..l];
        if mood {
            let _ = ifw.send(buf);
        } else {
            let _ = del.send((Instant::now() + Duration::from_secs(1), buf.to_vec()));
        }
        mood = ! mood;
    }
}

fn delayline(if_: &Iface, r: Receiver<(Instant, Vec<u8>)>) {
    while let Ok((t, b)) = r.recv() {
        std::thread::sleep(t.saturating_duration_since(Instant::now()));
        let _ = if_.send(&b[..]);
    }
}

fn main() -> anyhow::Result<()> {
    let if1 = Iface:: without_packet_info("tun1", Mode::Tun)?;
    let if2 = Iface:: without_packet_info("tun2", Mode::Tun)?;

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
    });


    Ok(())
}
