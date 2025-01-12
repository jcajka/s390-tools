// SPDX-License-Identifier: MIT
//
// Copyright IBM Corp. 2023

use crate::{
    error::{bail_spec, file_error, path_to_str},
    Error, FileIoErrorType, Result,
};

#[cfg(feature = "request")]
use openssl::x509::X509Crl;
#[cfg(feature = "request")]
use openssl::x509::X509;
use std::io::{Read, Write};
use std::path::Path;
use zerocopy::{AsBytes, BigEndian, FromBytes, U64};

/// Asserts a constant expression evaluates to `true`.
///
/// If the expression is not evaluated to `true` the compilation will fail.
#[macro_export]
macro_rules! static_assert {
    ($condition:expr) => {
        const _: () = core::assert!($condition);
    };
}

/// Asserts that a type has a specific size.
///
/// Useful to validate structs that are passed to C code.
/// If the expression is not evaluated to `true` the compilation will fail.
///
/// # Example
/// ```rust
/// # use pv::assert_size;
/// # fn main() {}
/// #[repr(C)]
/// struct c_struct {
///     v: u64,
/// }
/// assert_size!(c_struct, 8);
/// // assert_size!(c_struct, 7);//won't compile
/// ```
#[macro_export]
macro_rules! assert_size {
    ($t:ty, $sz:expr ) => {
        $crate::static_assert!(::std::mem::size_of::<$t>() == $sz);
    };
}

/// Trait that describes bitflags, represented by `T`.
pub trait Flags<T>: From<T> + for<'a> From<&'a T> {
    /// Set the specified bit to one.
    /// # Panics
    ///Panics if bit is >= 64
    fn set_bit(&mut self, bit: u8);
    /// Set the specified bit to zero.
    /// # Panics
    ///Panics if bit is >= 64
    fn unset_bit(&mut self, bit: u8);
    /// Test if the specified bit is set.
    /// # Panics
    ///Panics if bit is >= 64
    fn is_set(&self, bit: u8) -> bool;
}

/// Bitflags in MSB0 ordering
///
/// Wraps an u64 to set/get individual bits
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, AsBytes, FromBytes)]
pub struct Msb0Flags64(U64<BigEndian>);
impl Flags<u64> for Msb0Flags64 {
    #[track_caller]
    fn set_bit(&mut self, bit: u8) {
        assert!(bit < 64, "Flag bit set to greater than 63");
        let mut v = self.0.get();
        v |= 1 << (63 - bit);
        self.0.set(v)
    }

    #[track_caller]
    fn unset_bit(&mut self, bit: u8) {
        assert!(bit < 64, "Flag bit set to greater than 63");
        let mut v = self.0.get();
        v &= !(1 << (63 - bit));
        self.0.set(v)
    }

    #[track_caller]
    fn is_set(&self, bit: u8) -> bool {
        assert!(bit < 64, "Flag bit set to greater than 63");
        self.0.get() & (1 << (63 - bit)) > 0
    }
}

impl From<u64> for Msb0Flags64 {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl From<&u64> for Msb0Flags64 {
    fn from(value: &u64) -> Self {
        (*value).into()
    }
}

/// Bitflags in LSB0 ordering
///
/// Wraps an u64 to set/get individual bits
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, AsBytes, FromBytes)]
pub struct Lsb0Flags64(U64<BigEndian>);
impl Flags<u64> for Lsb0Flags64 {
    #[track_caller]
    fn set_bit(&mut self, bit: u8) {
        assert!(bit < 64, "Flag bit set to greater than 63");
        let mut v = self.0.get();
        v |= 1 << bit;
        self.0.set(v)
    }

    #[track_caller]
    fn unset_bit(&mut self, bit: u8) {
        assert!(bit < 64, "Flag bit set to greater than 63");
        let mut v = self.0.get();
        v &= !(1 << bit);
        self.0.set(v)
    }

    #[track_caller]
    fn is_set(&self, bit: u8) -> bool {
        assert!(bit < 64, "Flag bit set to greater than 63");
        self.0.get() & (1 << bit) > 0
    }
}

impl From<u64> for Lsb0Flags64 {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl From<&u64> for Lsb0Flags64 {
    fn from(value: &u64) -> Self {
        (*value).into()
    }
}

/// Tries to convert a BE hex string into a 128 unsigned integer
/// The hexstring must contain 32chars of hexdigits
///
/// * `hex_str` - string to convert  can be prepended with "0x"
/// * `ctx` - Error context string in case of an error
/// ```rust
/// # use std::error::Error;
/// # use pv::misc::try_parse_u128;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let hex = "11223344556677889900aabbccddeeff";
/// try_parse_u128(&hex, "The test")?;
/// #     Ok(())
/// # }
/// ```
///
/// # Errors
/// If `hex_string` is not a 32 byte hex string an Error appears
pub fn try_parse_u128(hex_str: &str, ctx: &str) -> Result<[u8; 16]> {
    let hex_str = if hex_str.starts_with("0x") {
        hex_str.split_at(2).1
    } else {
        hex_str
    };
    if hex_str.len() != 32 {
        bail_spec!(format!(
            "{ctx} hexstring must be 32chars long to cover all 16 bytes"
        ));
    }
    parse_hex(hex_str).try_into().map_err(|_| {
        Error::Specification(format!(
            "{ctx} hexstring must be 32chars long to cover all 16 bytes"
        ))
    })
}

/// Tries to convert a BE hex string into a 64 unsigned integer
/// The hexstring must *NOT* contain 16 chars of hexdigits, but
/// 16 chars at most.
///
/// * `hex_str` - string to convert  can be prepended with "0x"
/// * `ctx` - Error context string in case of an error
/// ```rust
/// # use std::error::Error;
/// # use pv::misc::try_parse_u64;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let hex = "1234567890abcdef";
/// try_parse_u64(&hex, "The test")?;
/// #     Ok(())
/// # }
/// ```
///
/// # Errors
/// If `hex_string` is not a 32 byte hex string an Error appears
pub fn try_parse_u64(hex_str: &str, ctx: &str) -> Result<u64> {
    let hex_str = if hex_str.starts_with("0x") {
        hex_str.split_at(2).1
    } else {
        hex_str
    };
    if hex_str.len() > 16 {
        bail_spec!(format!(
            "{ctx} hexstring {hex_str} must be max 16 chars long"
        ));
    }
    Ok(u64::from_str_radix(hex_str, 16)?)
}

/// Read exactly COUNT bytes into the buffer.
///
/// * `path` - Path to file
/// * `ctx` - Error context string in case of an error
///
/// # Errors
/// If this function encounters an "end of file" before completely filling
/// the buffer, it returns an error. The contents of `buf` are unspecified in this case.
///
/// If any other read error is encountered then this function immediately
/// returns. The contents of `buf` are unspecified in this case.
///
/// If this function returns an error, it is unspecified how many bytes it
/// has read, but it will never read more than would be necessary to
/// completely fill the buffer.
pub fn read_exact_file<P: AsRef<Path>, const COUNT: usize>(
    path: P,
    ctx: &str,
) -> Result<[u8; COUNT]> {
    let mut f = std::fs::File::open(&path).map_err(|e| Error::FileAccess {
        ty: crate::FileAccessErrorType::Open,
        path: path_to_str!(path).to_string(),
        source: e,
    })?;

    if f.metadata()?.len() as usize != COUNT {
        bail_spec!(format!("{ctx} must be exactly {COUNT} bytes long"));
    }

    let mut buf = [0; COUNT];
    f.read_exact(&mut buf)
        .map_err(|e| file_error!(Read, ctx, path_to_str!(path).to_string(), e))?;
    Ok(buf)
}

/// Read content from a file and add context in case of an error
///
/// * `path` - Path to file
/// * `ctx` - Error context string in case of an error
///
///
/// # Errors
/// Passes through any kind of error `std::fs::read` produces
pub fn read_file<P: AsRef<Path>>(path: P, ctx: &str) -> Result<Vec<u8>> {
    std::fs::read(&path).map_err(|e| {
        file_error!(
            Read,
            ctx,
            path.as_ref().to_str().unwrap_or("no UTF-8 path"),
            e
        )
    })
}

/// Reads all content from a [`std::io::Read`] and add context in case of an error
///
/// * `path` - Path to file
/// * `ctx` - Error context string in case of an error
///
///
/// # Errors
/// Passes through any kind of error `std::fs::write` produces
pub fn read<R: Read>(rd: &mut R, path: &str, ctx: &str) -> Result<Vec<u8>> {
    let mut buf = vec![];
    rd.read_to_end(&mut buf).map_err(|e| Error::FileIo {
        ty: FileIoErrorType::Write,
        ctx: ctx.to_string(),
        path: path.to_string(),
        source: e,
    })?;
    Ok(buf)
}

/// write content to a file and add context in case of an error
///
/// * `path` - Path to file
/// * `ctx` - Error context string in case of an error
///
///
/// # Errors
/// Passes through any kind of error `std::fs::write` produces
pub fn write_file<D: AsRef<[u8]>>(path: &str, data: D, ctx: &str) -> Result<()> {
    std::fs::write(path, data.as_ref()).map_err(|e| Error::FileIo {
        ty: FileIoErrorType::Write,
        ctx: ctx.to_string(),
        path: path.to_string(),
        source: e,
    })
}

/// Write content to a [`std::io::Write`] and add context in case of an error
///
/// * `path` - Path to file
/// * `ctx` - Error context string in case of an error
///
///
/// # Errors
/// Passes through any kind of error `std::fs::write` produces
pub fn write<D: AsRef<[u8]>, W: Write>(wr: &mut W, data: D, path: &str, ctx: &str) -> Result<()> {
    wr.write_all(data.as_ref()).map_err(|e| Error::FileIo {
        ty: FileIoErrorType::Write,
        ctx: ctx.to_string(),
        path: path.to_string(),
        source: e,
    })
}

/// Read all CRLs from the buffer and parse them into a vector.
///
/// # Errors
///
/// This function will return an error if the underlying openssl implementation cannot parse `buf`
/// as `DER` or `PEM`.
///
/// Requires the `request` feature.
#[cfg(feature = "request")]
pub fn read_crls(buf: &[u8]) -> Result<Vec<X509Crl>> {
    use openssl_extensions::crl::StackableX509Crl;
    X509Crl::from_der(buf)
        .map(|crl| vec![crl])
        .or_else(|_| StackableX509Crl::stack_from_pem(buf))
        .map_err(Error::Crypto)
}

/// Read all certificates from the buffer and parse them into a vector.
///
/// # Errors
///
/// This function will return an error if the underlying openssl implementation cannot parse `buf`
/// as `DER` or `PEM`.
///
/// Requires the `request` feature.
#[cfg(feature = "request")]
pub fn read_certs(buf: &[u8]) -> Result<Vec<X509>> {
    X509::from_der(buf)
        .map(|crt| vec![crt])
        .or_else(|_| X509::stack_from_pem(buf))
        .map_err(Error::Crypto)
}

macro_rules! usize_to_ui {
    ($(#[$attr:meta])* => $t: ident, $name:ident) => {
        ///Converts an [`usize`] to an [`
        $(#[$attr])*
        ///`] if possible
        pub fn $name(u: usize) -> Option<$t> {
            if u > $t::MAX as usize {
                None
            } else {
                Some(u as $t)
            }
        }

    }
}

usize_to_ui! {
#[doc = r"u32"]
=> u32, to_u32}
usize_to_ui! {
#[doc = r"u16"]
=> u16, to_u16}

/// Test if both slices contain the exact same bytes.
///
/// Do not use this to compare cryptographic values (i.e. hashes)
pub fn memeq(lhs: &[u8], rhs: &[u8]) -> bool {
    let size = lhs.len();

    size == rhs.len()
        && unsafe {
            let l = lhs as *const _ as _;
            let r = rhs as *const _ as _;
            (l as usize) == (r as usize) || libc::memcmp(l, r, size) == 0
        }
}

/// Converts the hexstring into a byte vector.
///
/// Stops if the end or until a non hex chat is found
pub fn parse_hex(hex_str: &str) -> Vec<u8> {
    let mut hex_bytes = hex_str.as_bytes().iter().map_while(|b| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    });

    let mut bytes = Vec::new();
    while let (Some(h), Some(l)) = (hex_bytes.next(), hex_bytes.next()) {
        bytes.push(h << 4 | l)
    }
    bytes
}
/// Report if the `prot_virt_guest` sysfs entry is one.
///
/// If the entry does not exist returns false.
///
/// for non-s390-architectures:
/// Returns always false
/// A non-s390 system cannot be a secure execution guest.
#[allow(unreachable_code)]
pub fn pv_guest_bit_set() -> bool {
    #[cfg(not(target_arch = "s390x"))]
    return false;
    //s390 branch
    let v = std::fs::read("/sys/firmware/uv/prot_virt_guest").unwrap_or_else(|_| vec![0]);
    let v: u8 = String::from_utf8_lossy(&v[..1]).parse().unwrap_or(0);
    v == 1
}

#[cfg(test)]
mod tests {
    use std::usize;

    use super::*;
    #[cfg(feature = "request")]
    use crate::test_utils::*;

    #[test]
    fn msb_flags() {
        let v = 17;
        let v_flag: Msb0Flags64 = v.into();
        assert_eq!(v, v_flag.0.get());

        let mut v: Msb0Flags64 = 4.into();
        v.unset_bit(61);
        assert_eq!(v.0.get(), 0);
        v.set_bit(61);
        assert_eq!(4, v.0.get());

        let mut v = Msb0Flags64::default();
        v.set_bit(0);
        assert_eq!(&[0x80, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());
        v.set_bit(0);
        assert_eq!(&[0x80, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());
        v.set_bit(1);
        assert_eq!(&[0xc0, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());
        v.set_bit(2);
        assert_eq!(&[0xe0, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());
        v.set_bit(3);
        assert_eq!(&[0xf0, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());

        v.unset_bit(3);
        assert_eq!(&[0xe0, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());
        v.unset_bit(3);
        assert_eq!(&[0xe0, 0, 0, 0, 0, 0, 0, 0], v.as_bytes());

        v.set_bit(16);
        assert_eq!(&[0xe0, 0, 0x80, 0, 0, 0, 0, 0], v.as_bytes());
    }

    #[test]
    #[should_panic]
    fn msb_flags_set_panic() {
        Msb0Flags64::default().set_bit(64)
    }

    #[test]
    #[should_panic]
    fn msb_flags_unset_panic() {
        Msb0Flags64::default().unset_bit(64)
    }

    #[test]
    fn lsb_flags() {
        let v = 17;
        let v_flag: Lsb0Flags64 = v.into();
        assert_eq!(v, v_flag.0.get());

        let mut v: Lsb0Flags64 = 4.into();
        v.unset_bit(2);
        assert_eq!(v.0.get(), 0);
        v.set_bit(2);
        assert_eq!(4, v.0.get());

        let mut v = Lsb0Flags64::default();
        v.set_bit(0);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 1], v.as_bytes());
        v.set_bit(0);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 1], v.as_bytes());
        v.set_bit(1);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 3], v.as_bytes());
        v.set_bit(2);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 7], v.as_bytes());
        v.set_bit(3);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 0xf], v.as_bytes());

        v.unset_bit(3);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 7], v.as_bytes());
        v.unset_bit(3);
        assert_eq!(&[0, 0, 0, 0, 0, 0, 0, 7], v.as_bytes());

        v.set_bit(16);
        assert_eq!(&[0, 0, 0, 0, 0, 1, 0, 7], v.as_bytes());
    }

    #[test]
    #[should_panic]
    fn lsb_flags_set_panic() {
        Lsb0Flags64::default().set_bit(64)
    }

    #[test]
    #[should_panic]
    fn lsb_flags_unset_panic() {
        Lsb0Flags64::default().unset_bit(64)
    }
    #[test]
    fn parse_hex() {
        let s = "123456acbef0";
        let exp = vec![0x12, 0x34, 0x56, 0xac, 0xbe, 0xf0];
        assert_eq!(super::parse_hex(s), exp);

        let s = "00123456acbef0";
        let exp = vec![0, 0x12, 0x34, 0x56, 0xac, 0xbe, 0xf0];
        assert_eq!(super::parse_hex(s), exp);

        let s = "00123456acbef0ii90";
        let exp = vec![0, 0x12, 0x34, 0x56, 0xac, 0xbe, 0xf0];
        assert_eq!(super::parse_hex(s), exp);
    }

    #[test]
    #[cfg(feature = "request")]
    fn read_crls() {
        let crl = get_cert_asset("ibm.crl");
        let crl_der = get_cert_asset("der.crl");
        let fail = get_cert_asset("ibm.crt");
        assert_eq!(super::read_crls(&crl).unwrap().len(), 1);
        assert_eq!(super::read_crls(&crl_der).unwrap().len(), 1);
        assert_eq!(super::read_crls(&fail).unwrap().len(), 0);
    }

    #[test]
    #[cfg(feature = "request")]
    fn read_certs() {
        let crt = get_cert_asset("ibm.crt");
        let crt_der = get_cert_asset("der.crt");
        let fail = get_cert_asset("ibm.crl");
        assert_eq!(super::read_certs(&crt).unwrap().len(), 1);
        assert_eq!(super::read_certs(&crt_der).unwrap().len(), 1);
        assert_eq!(super::read_certs(&fail).unwrap().len(), 0);
    }

    #[test]
    fn to_u32() {
        assert_eq!(Some(17), super::to_u32(17));
        assert_eq!(Some(0), super::to_u32(0));
        assert_eq!(Some(u32::MAX), super::to_u32(u32::MAX as usize));
        assert_eq!(None, super::to_u32(u32::MAX as usize + 1));
        assert_eq!(None, super::to_u32(usize::MAX));
    }

    #[test]
    fn parse_u128() {
        assert!(matches!(
            try_parse_u128("123456", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("-1234", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("0011223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("dd11223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("-1223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));

        assert!(matches!(
            try_parse_u128("0x123456", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("-0x1234", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("0x0011223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("0xdd11223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));
        assert!(matches!(
            try_parse_u128("0x-1223344556677889900aabbccddeeff", ""),
            Err(Error::Specification(_))
        ));

        assert_eq!(
            [
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ],
            try_parse_u128("11223344556677889900aabbccddeeff", "").unwrap()
        );
        assert_eq!(
            [
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ],
            try_parse_u128("0x11223344556677889900aabbccddeeff", "").unwrap()
        );
        assert_eq!(
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ],
            try_parse_u128("00112233445566778899aabbccddeeff", "").unwrap()
        );
        assert_eq!(
            [
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
                0xee, 0xff
            ],
            try_parse_u128("00112233445566778899aabbccddeeff", "").unwrap()
        );
    }

    #[test]
    fn memeq() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        let b = [1, 2, 3, 4, 5, 6, 7, 8, 9, 1];
        let c = [0, 0, 1, 2, 3, 4];

        assert!(super::memeq(&a, &a));
        assert!(super::memeq(&a, &a.clone()));
        assert!(!super::memeq(&b, &a));
        assert!(!super::memeq(&b, &c));
        assert!(!super::memeq(&b, &[]));
    }
}
