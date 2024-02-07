use libloc::Locations;
use std::env;
use std::net::IpAddr;

fn main() {
    let mut args = env::args();
    args.next();
    let args: Vec<_> = args.collect();

    let locations = Locations::open("/usr/share/libloc-location/location.db").unwrap();
    //let locations = Locations::open("/tmp/location.db").unwrap();
    if args.is_empty() {
        println!("created_at: {}", locations.created_at());
        println!("\nvendor:\n{}", locations.vendor());
        println!("\ndescription:\n{}", locations.description());
        println!("\nlicense:\n{}", locations.license());
    } else {
        for arg in args {
            let addr: IpAddr = match arg.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    eprintln!("{}: invalid IP address: {}", arg, e);
                    continue;
                }
            };
            match locations.lookup(addr) {
                Some(network) => {
                    let as_name = locations
                        .as_(network.asn())
                        .map(|as_| as_.name())
                        .unwrap_or("AS name unknown");
                    let country = locations.country(network.country_code()).expect("country");
                    println!(
                        "{} ({}): AS{}, {}, {}:{}, {}",
                        addr,
                        network.addrs(),
                        network.asn(),
                        as_name,
                        country.continent_code(),
                        country.code(),
                        country.name()
                    );
                }
                None => println!("{}: unknown", addr),
            }
        }
    }
}
