use bencher::benchmark_group;
use bencher::benchmark_main;
use bencher::black_box;
use bencher::Bencher;
use libloc::Locations;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

const PATH: &str = "/usr/share/libloc-location/location.db";
const ADDR: &str = "1.1.1.1";

fn locations() -> Locations {
    Locations::open(PATH).unwrap()
}

fn open(bench: &mut Bencher) {
    bench.iter(|| {
        locations();
    });
}

fn lookup(bench: &mut Bencher) {
    let locations = locations();
    let addr: IpAddr = ADDR.parse().unwrap();
    bench.iter(|| {
        black_box(locations.lookup(black_box(addr)));
    });
}

fn lookup_v4(bench: &mut Bencher) {
    let locations = locations();
    let addr: Ipv4Addr = ADDR.parse().unwrap();
    bench.iter(|| {
        black_box(locations.lookup_v4(black_box(addr)));
    });
}

fn lookup_v6(bench: &mut Bencher) {
    let locations = locations();
    let addr: Ipv4Addr = ADDR.parse().unwrap();
    let addr: Ipv6Addr = addr.to_ipv6_mapped();
    bench.iter(|| {
        black_box(locations.lookup_v6(black_box(addr)));
    });
}

#[rustfmt::skip]
benchmark_group!(locations_main,
    open,
    lookup,
    lookup_v4,
    lookup_v6,
);
benchmark_main!(locations_main);
