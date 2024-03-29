use bencher::benchmark_main;

#[cfg(not(windows))]
mod inner {
    use bencher::benchmark_group;
    use bencher::black_box;
    use bencher::Bencher;
    use std::ffi::CStr;
    use std::net::Ipv4Addr;
    use std::net::Ipv6Addr;
    use std::ptr;

    static PATH: &CStr = match CStr::from_bytes_with_nul(b"location.db\0") {
        Ok(p) => p,
        Err(_) => unreachable!(),
    };
    static MODE: &CStr = match CStr::from_bytes_with_nul(b"r\0") {
        Ok(m) => m,
        Err(_) => unreachable!(),
    };
    static ADDR: &str = "1.1.1.1";

    #[allow(bad_style)]
    mod sys {
        use libc::c_int;
        use libc::in6_addr;
        use libc::FILE;

        #[non_exhaustive]
        pub enum loc_ctx {}

        #[non_exhaustive]
        pub enum loc_database {}

        #[non_exhaustive]
        pub enum loc_network {}

        #[link(name = "loc")]
        extern "C" {
            pub fn loc_new(ctx: *mut *mut loc_ctx) -> c_int;
            pub fn loc_unref(ctx: *mut loc_ctx) -> *mut loc_ctx;

            pub fn loc_database_new(
                ctx: *mut loc_ctx,
                database: *mut *mut loc_database,
                f: *mut FILE,
            ) -> c_int;
            pub fn loc_database_unref(database: *mut loc_database) -> *mut loc_database;

            pub fn loc_database_lookup(
                db: *mut loc_database,
                address: *const in6_addr,
                network: *mut *mut loc_network,
            ) -> c_int;
            pub fn loc_network_unref(network: *mut loc_network) -> *mut loc_network;
        }
    }

    struct File(*mut libc::FILE);

    impl File {
        fn open(path: &CStr, mode: &CStr) -> File {
            let result;
            unsafe {
                result = libc::fopen(path.as_ptr(), mode.as_ptr());
            }
            assert!(!result.is_null());
            File(result)
        }
    }

    impl Drop for File {
        fn drop(&mut self) {
            unsafe {
                libc::fclose(self.0);
            }
        }
    }

    struct Context(*mut sys::loc_ctx);

    impl Context {
        fn new() -> Context {
            let mut result = ptr::null_mut();
            unsafe {
                sys::loc_new(&mut result);
            }
            assert!(!result.is_null());
            Context(result)
        }
    }

    impl Drop for Context {
        fn drop(&mut self) {
            unsafe {
                sys::loc_unref(self.0);
            }
        }
    }

    struct Database(*mut sys::loc_database);

    impl Database {
        fn new(ctx: &mut Context, f: &mut File) -> Database {
            let mut result = ptr::null_mut();
            unsafe {
                sys::loc_database_new(ctx.0, &mut result, f.0);
            }
            assert!(!result.is_null());
            Database(result)
        }
        fn lookup_v6(&mut self, address: &libc::in6_addr) -> Option<Network> {
            let mut result = ptr::null_mut();
            unsafe {
                sys::loc_database_lookup(self.0, address, &mut result);
            }
            if !result.is_null() {
                Some(Network(result))
            } else {
                None
            }
        }
    }

    impl Drop for Database {
        fn drop(&mut self) {
            unsafe {
                sys::loc_database_unref(self.0);
            }
        }
    }

    struct Network(*mut sys::loc_network);

    impl Drop for Network {
        fn drop(&mut self) {
            unsafe {
                sys::loc_network_unref(self.0);
            }
        }
    }

    fn database(context: &mut Context) -> Database {
        Database::new(context, &mut File::open(PATH, MODE))
    }

    fn open(bench: &mut Bencher) {
        let mut context = Context::new();
        bench.iter(|| {
            database(&mut context);
        });
    }

    fn lookup_v6(bench: &mut Bencher) {
        let mut context = Context::new();
        let mut database = database(&mut context);
        let addr: Ipv4Addr = ADDR.parse().unwrap();
        let addr: Ipv6Addr = addr.to_ipv6_mapped();
        let addr = libc::in6_addr {
            s6_addr: addr.octets(),
        };
        bench.iter(|| {
            black_box(database.lookup_v6(black_box(&addr)));
        })
    }

    #[rustfmt::skip]
    benchmark_group!(native_main,
        open,
        lookup_v6,
    );
}

#[cfg(windows)]
mod inner {
    use std::iter;

    pub fn native_main<T>() -> iter::Empty<T> {
        iter::empty()
    }
}

benchmark_main!(inner::native_main);
