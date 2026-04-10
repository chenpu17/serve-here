use crate::format::format_url_host;

pub fn print_listening_addresses(host: &str, port: u16) {
    if host == "0.0.0.0" || host == "::" {
        println!("  http://localhost:{}/", port);

        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            let mut seen = std::collections::HashSet::new();
            for (_name, ip) in interfaces {
                if ip.is_ipv4() && !ip.is_loopback() {
                    let addr = format!("  http://{}:{}/", ip, port);
                    if seen.insert(addr.clone()) {
                        println!("{}", addr);
                    }
                }
            }
        }
    } else {
        println!("  http://{}:{}/", format_url_host(host), port);
    }
}
