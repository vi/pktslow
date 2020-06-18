use tun_tap::{Iface,Mode};

use crossbeam_utils::thread::scope;

fn copy(if1: &Iface, if2: &Iface) {
    let mut buf = [0u8; 4096];
    while let Ok(l) = if1.recv(&mut buf[..]) {
        let buf : &[u8] = & buf[0..l];
        let _ = if2.send(buf);
    }
}

fn main() -> anyhow::Result<()> {
    let if1 = Iface:: without_packet_info("tun1", Mode::Tun)?;
    let if2 = Iface:: without_packet_info("tun2", Mode::Tun)?;

    let _ = scope(|s|  {
        let _h1 = s.spawn(|_| {
            copy(&if1, &if2);
        });
        let _h2 = s.spawn(|_| {
            copy(&if2, &if1);
        });
    });


    Ok(())
}
