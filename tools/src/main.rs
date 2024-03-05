use clap::Parser;
use libloc::Locations;
use std::net::IpAddr;
use std::path::PathBuf;

/// Look up an IP addres in a libloc database.
#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    /// IP addresses to look up. If none are passed, show meta information
    /// about the database instead.
    ip_addrs: Vec<IpAddr>,

    /// Path to database.
    #[arg(long, default_value = "/usr/share/libloc-location/location.db")]
    database: PathBuf,
}

fn main() {
    let args = Args::parse();

    let locations = Locations::open(&args.database).unwrap();
    if args.ip_addrs.is_empty() {
        println!("created_at: {}", locations.created_at());
        println!("\nvendor:\n{}", locations.vendor());
        println!("\ndescription:\n{}", locations.description());
        println!("\nlicense:\n{}", locations.license());
    } else {
        for addr in args.ip_addrs {
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
