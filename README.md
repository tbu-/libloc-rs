Read location databases in [libloc] format.

It features constant-time lookups of IPv6/IPv4 addresses to [ASN] and country
data (["GeoIP"]). The database also contains names of [AS] and countries.

**The main struct of this crate is [`Locations`]**. First, you'll need a
database. A recent version of the official IPFire database can be obtained from
<https://location.ipfire.org/databases/1/location.db.xz>, xz-compressed. Then
you can start from [`Locations::open`] and use [`Locations::lookup`] to look up
information on a particular IP address.

A short overview about the internals of the database format can be found at
<https://www.ipfire.org/blog/libloc-or-what-is-working-inside-it>. A Kaitai
struct implementation `ipfire_libloc_db_v1.ksy` can be found in this
repository.

# Examples

```
use libloc::Locations;

let locations = Locations::open("example-location.db")?;

let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
assert_eq!(network.country_code(), "DE");
assert_eq!(network.asn(), 204867);
assert_eq!(network.is_anonymous_proxy(), false);
assert_eq!(network.addrs().to_string(), "2a07:1c44:5800::/40");

let country: libloc::Country = locations.country("DE").unwrap();
assert_eq!(country.continent_code(), "EU");
assert_eq!(country.name(), "Germany");

let as_: libloc::As = locations.as_(204867).unwrap();
assert_eq!(as_.name(), "Lightning Wire Labs GmbH");

# Ok::<(), libloc::OpenError>(())
```

# Panics

Any function from this library might panic if the database is corrupt.

# Benches

This library was written for fun. It's still a lot faster than the original
[libloc], even though it wasn't optimized for speed.

```text
     Running benches/locations.rs

running 4 tests
test lookup    ... bench:          87 ns/iter (+/- 15)
test lookup_v4 ... bench:          61 ns/iter (+/- 5)
test lookup_v6 ... bench:         396 ns/iter (+/- 22)
test open      ... bench:      64,012 ns/iter (+/- 4,592)

test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured

     Running benches/native.rs

running 2 tests
test lookup_v6 ... bench:       1,019 ns/iter (+/- 128)
test open      ... bench:     234,420 ns/iter (+/- 19,638)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured
```

It's looking up IPv6 addresses roughly 2.5x as fast, it has special handling
for IPv4 addresses which makes it >10x as fast.


["GeoIP"]: https://en.wikipedia.org/wiki/Internet_geolocation
[ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
[AS]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
[libloc]: https://www.ipfire.org/projects/location/
