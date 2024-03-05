#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use ipnet::IpNet;
use ipnet::Ipv4Net;
use ipnet::Ipv6Net;
use memmap2::Mmap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::path::Path;
use std::str;
use yoke::Yoke;
use yoke_derive::Yokeable;
use zerocopy::FromBytes;

mod format;

/// Error type for the [`Locations::open`] function.
#[derive(Debug)]
#[non_exhaustive]
pub enum OpenError {
    /// Error opening database file.
    ///
    /// The file might not exist or you might not have permissions to read it.
    ///
    /// The inner error is the one returned from [`std::fs::File::open`].
    Open(io::Error),
    /// Error memory-mapping database file.
    Mmap(io::Error),
    /// Invalid database file magic, likely not the correct format.
    InvalidMagic,
    /// Unsupported database version.
    UnsupportedVersion(u8),
    /// Couldn't read database file header, database corrupted.
    CouldntReadHeader,
    /// Invalid database header field: `as`, database corrupted.
    InvalidAsRange,
    /// Invalid database header field: `network`, database corrupted.
    InvalidNetworkRange,
    /// Invalid database header field: `network_node`, database corrupted.
    InvalidNetworkNodeRange,
    /// Invalid database header field: `country`, database corrupted.
    InvalidCountryRange,
    /// Invalid database header field: `string_pool`, database corrupted.
    InvalidStringPoolRange,
}

impl Error for OpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::OpenError::*;
        match self {
            Open(e) => Some(e),
            Mmap(e) => Some(e),
            InvalidMagic
            | UnsupportedVersion(_)
            | CouldntReadHeader
            | InvalidAsRange
            | InvalidNetworkRange
            | InvalidNetworkNodeRange
            | InvalidCountryRange
            | InvalidStringPoolRange => None,
        }
    }
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::OpenError::*;
        match self {
            Open(e) => write!(f, "error opening database file: {}", e),
            Mmap(e) => write!(f, "error memory-mapping database file: {}", e),
            InvalidMagic => "invalid database file magic, likely not the correct format".fmt(f),
            UnsupportedVersion(ver) => write!(f, "unsupported database version {}", ver),
            CouldntReadHeader => "couldn't read database file header, database corrupted".fmt(f),
            InvalidAsRange => "invalid database header field: as, database corrupted".fmt(f),
            InvalidNetworkRange => {
                "invalid database header field: network, database corrupted".fmt(f)
            }
            InvalidNetworkNodeRange => {
                "invalid database header field: network_node, database corrupted".fmt(f)
            }
            InvalidCountryRange => {
                "invalid database header field: country, database corrupted".fmt(f)
            }
            InvalidStringPoolRange => {
                "invalid database header field: string_pool, database corrupted".fmt(f)
            }
        }
    }
}

/// Information on an [AS] (autonomous system).
///
/// Returned by the [`Locations::as_`] function.
///
/// [AS]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
#[derive(Debug)]
pub struct As<'a> {
    asn: u32,
    name: &'a str,
}

/// Information on an IP network.
///
/// Returned by the [`Locations::lookup`] function.
#[derive(Debug)]
pub struct Network<'a> {
    inner: NetworkInner<'a>,
    addrs: IpNet,
}

/// Information on an IPv4 network.
///
/// See [`Network`].
#[derive(Debug)]
pub struct NetworkV4<'a> {
    inner: NetworkInner<'a>,
    addrs: Ipv4Net,
}

/// Information on an IPv6 network.
///
/// See [`Network`].
#[derive(Debug)]
pub struct NetworkV6<'a> {
    inner: NetworkInner<'a>,
    addrs: Ipv6Net,
}

#[derive(Debug)]
struct NetworkInner<'a> {
    // TODO: how to deal with XX? treat it as None?
    country_code: &'a str,
    // TODO: how to deal with AS0? treat it as None?
    asn: u32,
    flags: u16,
}

/// Information on a country.
///
/// Returned by the [`Locations::country`] function.
#[derive(Debug)]
pub struct Country<'a> {
    code: &'a str,
    continent_code: &'a str,
    name: &'a str,
}

impl<'a> As<'a> {
    fn from(inner: &LocationsInner<'a>, as_: &'a format::As) -> As<'a> {
        As {
            asn: as_.id.get(),
            name: inner.string(as_.name),
        }
    }
    /// The [ASN] (number) of the [AS].
    ///
    /// [AS]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    /// [ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    pub fn asn(&self) -> u32 {
        self.asn
    }
    /// The human-readable name of the AS.
    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a> NetworkInner<'a> {
    fn from(_inner: &LocationsInner<'a>, network: &'a format::Network) -> NetworkInner<'a> {
        NetworkInner {
            country_code: str::from_utf8(&network.country_code).unwrap_or_else(|e| {
                panic!(
                    "corrupt libloc db: invalid UTF-8 in network country code: {}",
                    e,
                );
            }),
            asn: network.asn.get(),
            flags: network.flags.get(),
        }
    }
}

impl<'a> Network<'a> {
    /// The [ISO 3166-1 alpha-2] country code of the country associated with
    /// this network.
    ///
    /// `"XX"` if unknown.
    ///
    /// [ISO 3166-1 alpha-2]: https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.country_code(), "DE");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn country_code(&self) -> &'a str {
        self.inner.country_code
    }
    /// The [ASN] of this network.
    ///
    /// 0 if unknown.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.asn(), 204867);
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    pub fn asn(&self) -> u32 {
        self.inner.asn
    }
    /// Whether the network hosts anonymous proxies.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.is_anonymous_proxy(), false);
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    pub fn is_anonymous_proxy(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANONYMOUS_PROXY != 0
    }
    /// Whether the network is a satellite provider.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.is_satellite_provider(), false);
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    pub fn is_satellite_provider(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_SATTELITE_PROVIDER != 0
    }
    /// Whether the network consists of [anycast] addresses.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.is_anycast(), true);
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [anycast]: https://en.wikipedia.org/wiki/Anycast
    pub fn is_anycast(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANYCAST != 0
    }
    #[allow(missing_docs)]
    pub fn is_drop(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_DROP != 0
    }
    /// All the addresses belonging to this particular network.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// let network: libloc::Network = locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap();
    /// assert_eq!(network.addrs().to_string(), "2a07:1c44:5800::/40");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [anycast]: https://en.wikipedia.org/wiki/Anycast
    pub fn addrs(&self) -> IpNet {
        self.addrs
    }
}

impl<'a> From<NetworkV4<'a>> for Network<'a> {
    fn from(network: NetworkV4<'a>) -> Network<'a> {
        Network {
            inner: network.inner,
            addrs: network.addrs.into(),
        }
    }
}

impl<'a> From<NetworkV6<'a>> for Network<'a> {
    fn from(network: NetworkV6<'a>) -> Network<'a> {
        Network {
            inner: network.inner,
            addrs: network.addrs.into(),
        }
    }
}

impl<'a> NetworkV4<'a> {
    /// See [`Network::country_code`].
    pub fn country_code(&self) -> &'a str {
        self.inner.country_code
    }
    /// See [`Network::asn`].
    pub fn asn(&self) -> u32 {
        self.inner.asn
    }
    /// See [`Network::is_anonymous_proxy`].
    pub fn is_anonymous_proxy(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANONYMOUS_PROXY != 0
    }
    /// See [`Network::is_satellite_provider`].
    pub fn is_satellite_provider(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_SATTELITE_PROVIDER != 0
    }
    /// See [`Network::is_anycast`].
    pub fn is_anycast(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANYCAST != 0
    }
    /// See [`Network::is_drop`].
    pub fn is_drop(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_DROP != 0
    }
    /// See [`Network::addrs`].
    pub fn addrs(&self) -> Ipv4Net {
        self.addrs
    }
}

impl<'a> NetworkV6<'a> {
    /// See [`Network::country_code`].
    pub fn country_code(&self) -> &'a str {
        self.inner.country_code
    }
    /// See [`Network::asn`].
    pub fn asn(&self) -> u32 {
        self.inner.asn
    }
    /// See [`Network::is_anonymous_proxy`].
    pub fn is_anonymous_proxy(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANONYMOUS_PROXY != 0
    }
    /// See [`Network::is_satellite_provider`].
    pub fn is_satellite_provider(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_SATTELITE_PROVIDER != 0
    }
    /// See [`Network::is_anycast`].
    pub fn is_anycast(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_ANYCAST != 0
    }
    /// See [`Network::is_drop`].
    pub fn is_drop(&self) -> bool {
        self.inner.flags & format::NETWORK_FLAG_DROP != 0
    }
    /// See [`Network::addrs`].
    pub fn addrs(&self) -> Ipv6Net {
        self.addrs
    }
}

impl<'a> Country<'a> {
    fn from(inner: &LocationsInner<'a>, country: &'a format::Country) -> Country<'a> {
        Country {
            code: str::from_utf8(&country.code).unwrap_or_else(|e| {
                panic!("corrupt libloc db: invalid UTF-8 in country code: {}", e);
            }),
            continent_code: str::from_utf8(&country.continent_code).unwrap_or_else(|e| {
                panic!(
                    "corrupt libloc db: invalid UTF-8 in country continent code: {}",
                    e,
                );
            }),
            name: inner.string(country.name),
        }
    }
    /// The [ISO 3166-1 alpha-2] code of the country.
    ///
    /// It consists of two uppercase latin letters.
    ///
    /// [ISO 3166-1 alpha-2]: https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2
    pub fn code(&self) -> &'a str {
        self.code
    }
    /// The [ISO 3166] code of the continent the country resides in.
    ///
    /// - `"AF"` for Africa.
    /// - `"AN"` for Antarctica.
    /// - `"AS"` for Asia.
    /// - `"EU"` for Europe.
    /// - `"NA"` for North America.
    /// - `"OC"` for Oceania.
    /// - `"SA"` for South America.
    ///
    /// [ISO 3166]: https://en.wikipedia.org/wiki/ISO_3166
    pub fn continent_code(&self) -> &'a str {
        self.continent_code
    }
    /// The human-readable name of the country in English.
    pub fn name(&self) -> &'a str {
        self.name
    }
}

/// A database in libloc format. **Main struct of this crate.**
pub struct Locations {
    inner: Yoke<LocationsInner<'static>, Mmap>,
}

#[derive(Yokeable)]
struct LocationsInner<'a> {
    header: &'a format::Header,
    as_: &'a [format::As],
    networks: &'a [format::Network],
    network_nodes: &'a [format::NetworkNode],
    countries: &'a [format::Country],
    string_pool: &'a [u8],
    ipv4_network_node: Option<u32>,
}

impl<'a> LocationsInner<'a> {
    fn find_network(&self, root: u32, bits_reverse: u128, num_bits: u32) -> Option<(u8, u32)> {
        // Walk the tree, remembering the last network we saw.
        let mut used_bits = 0;
        let mut bits = bits_reverse;
        let mut cur = self.network_node(root);
        let mut last_network = None;
        for _ in 0..num_bits {
            let next_index = cur.children[(bits & 1 != 0) as usize].get();
            if next_index == 0 {
                break;
            }
            last_network = cur.network().map(|n| (used_bits, n)).or(last_network);
            bits >>= 1;
            used_bits += 1;
            cur = self.network_node(next_index);
        }
        last_network = cur.network().map(|n| (used_bits, n)).or(last_network);
        last_network
    }
    fn find_network_node(&self, root: u32, bits_reverse: u128, num_bits: u32) -> Option<u32> {
        // Walk the tree.
        let mut bits = bits_reverse;
        let mut cur_index = root;
        for _ in 0..num_bits {
            cur_index = self.network_node(cur_index).children[(bits & 1 != 0) as usize].get();
            if cur_index == 0 {
                return None;
            }
            bits >>= 1;
        }
        Some(cur_index)
    }
    fn as_(&self, index: u32) -> &'a format::As {
        let index = index as usize;
        if index >= self.as_.len() {
            panic!(
                "corrupt libloc db: invalid as index: {} > {}",
                index,
                self.as_.len(),
            );
        }
        &self.as_[index]
    }
    fn network(&self, index: u32) -> &'a format::Network {
        let index = index as usize;
        if index >= self.networks.len() {
            panic!(
                "corrupt libloc db: invalid network index: {} > {}",
                index,
                self.networks.len(),
            );
        }
        &self.networks[index]
    }
    fn network_node(&self, index: u32) -> &'a format::NetworkNode {
        let index = index as usize;
        if index >= self.network_nodes.len() {
            panic!(
                "corrupt libloc db: invalid network node index: {} > {}",
                index,
                self.network_nodes.len(),
            );
        }
        &self.network_nodes[index]
    }
    fn country(&self, index: u32) -> &'a format::Country {
        let index = index as usize;
        if index >= self.countries.len() {
            panic!(
                "corrupt libloc db: invalid country index: {} > {}",
                index,
                self.countries.len(),
            );
        }
        &self.countries[index]
    }
    fn string(&self, str_ref: format::StrRef) -> &'a str {
        let offset = str_ref.offset.get() as usize;
        if offset > self.string_pool.len() {
            panic!(
                "corrupt libloc db: invalid str_ref: {} > {}",
                offset,
                self.string_pool.len(),
            );
        }
        let bytes = &self.string_pool[offset..];
        let bytes = &bytes[..bytes
            .iter()
            .copied()
            .position(|b| b == 0)
            .unwrap_or_else(|| {
                panic!(
                    "corrupt libloc db: missing null termination for str_ref: {}",
                    offset,
                );
            })];
        str::from_utf8(bytes).unwrap_or_else(|e| {
            panic!(
                "corrupt libloc db: invalid UTF-8 for str_ref: {}: {}",
                offset, e,
            )
        })
    }
}

trait ByteSliceExt {
    fn get_range(&self, range: format::FileRange) -> Option<&[u8]>;
    fn get_typed_range<T: FromBytes>(&self, range: format::FileRange) -> Option<&[T]>;
}
impl<'a> ByteSliceExt for [u8] {
    fn get_range(&self, range: format::FileRange) -> Option<&[u8]> {
        let start = range.offset.get();
        let end = range.offset.get().checked_add(range.length.get())?;
        self.get(start as usize..end as usize)
    }
    fn get_typed_range<T: FromBytes>(&self, range: format::FileRange) -> Option<&[T]> {
        self.get_range(range).and_then(T::slice_from)
    }
}

impl Locations {
    /// Open a database in libloc format.
    ///
    /// # Safety
    ///
    /// This memory-maps the database. This is efficient, but you must make
    /// sure that it's not modified during the usage. See the safety discussion
    /// of the `Mmap` struct of [`memmap2`](https://docs.rs/memmap2/).
    ///
    /// # Errors
    ///
    /// Errors can occur when the specified database file cannot be opened for
    /// reading (e.g. because it does not exist), this is communicated via the
    /// [`OpenError::Open`] variant.
    ///
    /// Additionally, if the opened file is not in a format valid for this
    /// crate, it is likely that the [`OpenError::InvalidMagic`] variant is
    /// returned.
    ///
    /// If the database is obviously corrupt, e.g. truncated, other errors
    /// might be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    ///
    /// // IO errors while opening the file are reported via the `Open(_)`
    /// // variant.
    /// assert!(matches!(Locations::open("non-existing"), Err(libloc::OpenError::Open(_))));
    ///
    /// // Files that are not in the required format are likely to give the
    /// // `InvalidMagic` error.
    /// assert!(matches!(Locations::open("Cargo.toml"), Err(libloc::OpenError::InvalidMagic)));
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Locations, OpenError> {
        fn inner(path: &Path) -> Result<Locations, OpenError> {
            use self::OpenError as Error;
            let file = File::open(path).map_err(Error::Open)?;
            let mmap = unsafe { Mmap::map(&file) }.map_err(Error::Mmap)?;

            if !mmap.starts_with(&format::MAGIC) {
                return Err(Error::InvalidMagic);
            }

            // This is just an optimization, ignore errors.
            #[cfg(unix)]
            let _ = mmap.advise(memmap2::Advice::Random);

            let inner = Yoke::try_attach_to_cart(mmap, |mmap| -> Result<_, Error> {
                let header =
                    format::Header::ref_from_prefix(&mmap).ok_or(Error::CouldntReadHeader)?;
                if header.version != format::VERSION {
                    return Err(Error::UnsupportedVersion(header.version));
                }

                let mut inner = LocationsInner {
                    as_: mmap
                        .get_typed_range(header.as_)
                        .ok_or(Error::InvalidAsRange)?,
                    networks: mmap
                        .get_typed_range(header.networks)
                        .ok_or(Error::InvalidNetworkRange)?,
                    network_nodes: mmap
                        .get_typed_range(header.network_nodes)
                        .ok_or(Error::InvalidNetworkNodeRange)?,
                    countries: mmap
                        .get_typed_range(header.countries)
                        .ok_or(Error::InvalidCountryRange)?,
                    string_pool: mmap
                        .get_range(header.string_pool)
                        .ok_or(Error::InvalidStringPoolRange)?,

                    header,

                    ipv4_network_node: Some(u32::MAX), // invalid value
                };
                let ipv4_mapped_prefix = u128::from(Ipv4Addr::from(0).to_ipv6_mapped());
                inner.ipv4_network_node =
                    inner.find_network_node(0, ipv4_mapped_prefix.reverse_bits(), 96);
                Ok(inner)
            })?;
            Ok(Locations { inner })
        }
        inner(path.as_ref())
    }
    /// The database creation time.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.created_at().to_string(), "2024-02-06 22:30:29 UTC");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn created_at(&self) -> chrono::DateTime<chrono::offset::Utc> {
        let inner = self.inner.get();
        let created_at = inner.header.created_at.get();
        chrono::DateTime::from_timestamp(
            created_at.try_into().unwrap_or_else(|_| {
                panic!(
                    "corrupt libloc db: invalid created_at header: {}",
                    created_at,
                )
            }),
            0,
        )
        .unwrap_or_else(|| {
            panic!(
                "corrupt libloc db: invalid created_at header: {}",
                created_at,
            )
        })
    }
    /// The vendor of the database.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.vendor(), "IPFire Project");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn vendor(&self) -> &str {
        let inner = self.inner.get();
        inner.string(inner.header.vendor)
    }
    /// The description of the database.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.description(), "This is a geo location database");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn description(&self) -> &str {
        let inner = self.inner.get();
        inner.string(inner.header.description)
    }
    /// The license of the database.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.license(), "CC");
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn license(&self) -> &str {
        let inner = self.inner.get();
        inner.string(inner.header.license)
    }
    /// Look up an [AS] (autonomous system) by its [ASN] (number).
    ///
    /// Returns `None` if it does not appear in the database.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.as_(204867).unwrap().name(), "Lightning Wire Labs GmbH");
    /// assert!(matches!(locations.as_(0), None));
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    ///
    /// [AS]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    /// [ASN]: https://en.wikipedia.org/wiki/Autonomous_system_(Internet)
    pub fn as_(&self, asn: u32) -> Option<As<'_>> {
        let inner = self.inner.get();

        // The ASs are stored sorted by ASN in the database, so we can use a
        // binary search to find a particular one.
        let index = inner
            .as_
            .binary_search_by_key(&asn, |as_| as_.id.get())
            .ok()?;
        Some(As::from(inner, inner.as_(index.try_into().unwrap())))
    }
    /// Look up network information for an IP address.
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.lookup("2a07:1c44:5800::1".parse().unwrap()).unwrap().asn(), 204867);
    /// assert!(matches!(locations.lookup("127.0.0.1".parse().unwrap()), None));
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn lookup(&self, addr: IpAddr) -> Option<Network<'_>> {
        match addr {
            IpAddr::V4(addr) => self.lookup_v4(addr).map(Into::into),
            IpAddr::V6(addr) => self.lookup_v6(addr).map(Into::into),
        }
    }
    /// Look up network information for an IPv4 address.
    ///
    /// See [`Locations::lookup`].
    pub fn lookup_v4(&self, addr: Ipv4Addr) -> Option<NetworkV4<'_>> {
        let inner = self.inner.get();

        let (num_bits, network_idx) = inner.find_network(
            inner.ipv4_network_node?,
            u32::from(addr).reverse_bits().into(),
            32,
        )?;
        let addrs = Ipv4Net::new(addr, num_bits).unwrap().trunc();

        Some(NetworkV4 {
            inner: NetworkInner::from(inner, inner.network(network_idx)),
            addrs,
        })
    }
    /// Look up network information for an IPv6 address.
    ///
    /// See [`Locations::lookup`].
    pub fn lookup_v6(&self, addr: Ipv6Addr) -> Option<NetworkV6<'_>> {
        let inner = self.inner.get();

        let (num_bits, network_idx) =
            inner.find_network(0, u128::from(addr).reverse_bits(), 128)?;
        let addrs = Ipv6Net::new(addr, num_bits).unwrap().trunc();

        Some(NetworkV6 {
            inner: NetworkInner::from(inner, inner.network(network_idx)),
            addrs,
        })
    }
    /// Look up a country by its [ISO 3166-1 alpha-2] code.
    ///
    /// [ISO 3166-1 alpha-2]: https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2
    ///
    /// ```
    /// use libloc::Locations;
    ///
    /// let locations = Locations::open("example-location.db")?;
    /// assert_eq!(locations.country("DE").unwrap().name(), "Germany");
    /// assert!(matches!(locations.country("XX"), None));
    ///
    /// # Ok::<(), libloc::OpenError>(())
    /// ```
    pub fn country(&self, code: &str) -> Option<Country<'_>> {
        let inner = self.inner.get();

        if code.len() != 2 {
            return None;
        }
        let code = code.as_bytes();
        let code = [code[0], code[1]];
        // The countries are stored sorted by country code in the database, so
        // we can use a binary search to find a particular one.
        let index = inner
            .countries
            .binary_search_by_key(&code, |c| c.code)
            .ok()?;
        Some(Country::from(
            inner,
            inner.country(index.try_into().unwrap()),
        ))
    }
}
