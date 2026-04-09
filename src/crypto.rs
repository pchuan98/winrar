use crc32fast::Hasher as Crc32;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::{Rng, RngExt};
use sha1::{Digest, Sha1};
use std::sync::OnceLock;

// 有限域维度 17 个 15-bit 系数
const FIELD_DEGREE: usize = 17;
const LIMB_BITS: usize = 15;
const GF15_ORDER: usize = 0x7fff;

const MASTER_PRIVATE_KEY_HEX: &str = "59fe6abcca90bdb95f0105271fa85fb9f11f467450c1ae9044b7fd61d65e";
const ORDER_HEX: &str = "1026dd85081b82314691ced9bbec30547840e4bf72d8b5e0d258442bbcd31";

const G_X: [u16; FIELD_DEGREE] = [
    0x38CC, 0x052F, 0x2510, 0x45AA, 0x1B89, 0x4468, 0x4882, 0x0D67, 0x4FEB, 0x55CE, 0x0025, 0x4CB7,
    0x0CC2, 0x59DC, 0x289E, 0x65E3, 0x56FD,
];
const G_Y: [u16; FIELD_DEGREE] = [
    0x31A7, 0x65F2, 0x18C4, 0x3412, 0x7388, 0x54C1, 0x539B, 0x4A02, 0x4D07, 0x12D6, 0x7911, 0x3B5E,
    0x4F0E, 0x216F, 0x2BF2, 0x1974, 0x20DA,
];

#[cfg(test)]
const PUBLIC_KEY_X: [u16; FIELD_DEGREE] = [
    0x3A1A, 0x1109, 0x268A, 0x12F7, 0x3734, 0x75F0, 0x576C, 0x2EA4, 0x4813, 0x3F62, 0x0567, 0x784D,
    0x753D, 0x6D92, 0x366C, 0x1107, 0x3861,
];
#[cfg(test)]
const PUBLIC_KEY_Y: [u16; FIELD_DEGREE] = [
    0x6C20, 0x6027, 0x1B22, 0x7A87, 0x43C4, 0x1908, 0x2449, 0x4675, 0x7933, 0x2E66, 0x32F5, 0x2A58,
    0x1145, 0x74AC, 0x36D0, 0x2731, 0x12B6,
];

struct Gf15Tables {
    log: Box<[u16; 0x8000]>,
    exp: Box<[u16; 0x8000]>,
}

fn gf15_tables() -> &'static Gf15Tables {
    // 预计算 GF(2^15) 的 log / exp 表 提升乘法与求逆速度
    static TABLES: OnceLock<Gf15Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut log = Box::new([0u16; 0x8000]);
        let mut exp = Box::new([0u16; 0x8000]);

        exp[0] = 1;
        for i in 1..GF15_ORDER {
            let mut v = (exp[i - 1] as u32) << 1;
            if (v & 0x8000) != 0 {
                v ^= 0x8003;
            }
            exp[i] = v as u16;
        }
        for i in 0..GF15_ORDER {
            log[exp[i] as usize] = i as u16;
        }

        Gf15Tables { log, exp }
    })
}

fn gf15_mul(a: u16, b: u16) -> u16 {
    if a == 0 || b == 0 {
        return 0;
    }

    let tables = gf15_tables();
    let mut g = tables.log[a as usize] as usize + tables.log[b as usize] as usize;
    if g >= GF15_ORDER {
        g -= GF15_ORDER;
    }
    tables.exp[g]
}

fn gf15_square(a: u16) -> u16 {
    if a == 0 {
        return 0;
    }

    let tables = gf15_tables();
    let mut g = (tables.log[a as usize] as usize) * 2;
    if g >= GF15_ORDER {
        g -= GF15_ORDER;
    }
    tables.exp[g]
}

fn gf15_inv(a: u16) -> u16 {
    assert!(a != 0, "zero has no inverse");
    if a == 1 {
        return 1;
    }

    let tables = gf15_tables();
    let g = tables.log[a as usize] as usize;
    tables.exp[(GF15_ORDER - g) % GF15_ORDER]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Field([u16; FIELD_DEGREE]);

impl Field {
    const ZERO: Self = Self([0; FIELD_DEGREE]);

    fn from_limbs(limbs: [u16; FIELD_DEGREE]) -> Self {
        Self(limbs)
    }

    fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    fn add(&self, other: &Self) -> Self {
        let mut out = [0u16; FIELD_DEGREE];
        for (dst, (a, b)) in out.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *dst = *a ^ *b;
        }
        Self(out)
    }

    fn add_one(&self) -> Self {
        let mut out = self.0;
        out[0] ^= 1;
        Self(out)
    }

    fn mul(&self, other: &Self) -> Self {
        // 先做多项式乘法 再按 y^17 + y^3 + 1 约简
        let mut tmp = [0u16; FIELD_DEGREE * 2 - 1];

        for i in 0..FIELD_DEGREE {
            if self.0[i] == 0 {
                continue;
            }
            for j in 0..FIELD_DEGREE {
                if other.0[j] != 0 {
                    tmp[i + j] ^= gf15_mul(self.0[i], other.0[j]);
                }
            }
        }

        for i in (FIELD_DEGREE..tmp.len()).rev() {
            if tmp[i] != 0 {
                tmp[i - FIELD_DEGREE] ^= tmp[i];
                tmp[i - FIELD_DEGREE + 3] ^= tmp[i];
                tmp[i] = 0;
            }
        }

        let mut out = [0u16; FIELD_DEGREE];
        out.copy_from_slice(&tmp[..FIELD_DEGREE]);
        Self(out)
    }

    fn square(&self) -> Self {
        let mut tmp = [0u16; FIELD_DEGREE * 2 - 1];

        for i in 0..FIELD_DEGREE {
            tmp[i * 2] = gf15_square(self.0[i]);
        }

        for i in (FIELD_DEGREE..tmp.len()).rev() {
            if tmp[i] != 0 {
                tmp[i - FIELD_DEGREE] ^= tmp[i];
                tmp[i - FIELD_DEGREE + 3] ^= tmp[i];
                tmp[i] = 0;
            }
        }

        let mut out = [0u16; FIELD_DEGREE];
        out.copy_from_slice(&tmp[..FIELD_DEGREE]);
        Self(out)
    }

    fn inv(&self) -> Self {
        // 扩展欧几里得算法 求复合域元素逆元
        fn add_scale(
            dst: &mut [u16; FIELD_DEGREE * 2],
            deg_dst: &mut usize,
            alpha: u16,
            shift: usize,
            src: &[u16; FIELD_DEGREE * 2],
            deg_src: usize,
        ) {
            for i in 0..=deg_src {
                if src[i] == 0 {
                    continue;
                }
                let pos = i + shift;
                dst[pos] ^= gf15_mul(alpha, src[i]);
                if dst[pos] != 0 && pos > *deg_dst {
                    *deg_dst = pos;
                }
            }
            while *deg_dst > 0 && dst[*deg_dst] == 0 {
                *deg_dst -= 1;
            }
        }

        assert!(!self.is_zero(), "zero has no inverse");

        let mut b = [0u16; FIELD_DEGREE * 2];
        let mut c = [0u16; FIELD_DEGREE * 2];
        let mut f = [0u16; FIELD_DEGREE * 2];
        let mut g = [0u16; FIELD_DEGREE * 2];

        b[0] = 1;
        f[..FIELD_DEGREE].copy_from_slice(&self.0);
        g[0] = 1;
        g[3] = 1;
        g[17] = 1;

        let mut deg_b = 0usize;
        let mut deg_c = 0usize;
        let mut deg_f = self.0.iter().rposition(|&x| x != 0).unwrap();
        let mut deg_g = 17usize;

        loop {
            if deg_f == 0 {
                let inv = gf15_inv(f[0]);
                let mut out = [0u16; FIELD_DEGREE];
                for i in 0..=deg_b.min(FIELD_DEGREE - 1) {
                    if b[i] != 0 {
                        out[i] = gf15_mul(b[i], inv);
                    }
                }
                return Self(out);
            }

            if deg_f < deg_g {
                std::mem::swap(&mut f, &mut g);
                std::mem::swap(&mut b, &mut c);
                std::mem::swap(&mut deg_f, &mut deg_g);
                std::mem::swap(&mut deg_b, &mut deg_c);
            }

            let shift = deg_f - deg_g;
            let alpha = gf15_mul(f[deg_f], gf15_inv(g[deg_g]));
            add_scale(&mut f, &mut deg_f, alpha, shift, &g, deg_g);
            add_scale(&mut b, &mut deg_b, alpha, shift, &c, deg_c);
        }
    }

    fn div(&self, other: &Self) -> Self {
        self.mul(&other.inv())
    }

    fn lsb(&self) -> u8 {
        (self.0[0] & 1) as u8
    }

    fn to_biguint(&self) -> BigUint {
        let mut n = BigUint::zero();
        for limb in self.0.iter().rev() {
            n <<= LIMB_BITS;
            n += BigUint::from(*limb as u32);
        }
        n
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Point {
    x: Field,
    y: Field,
    inf: bool,
}

impl Point {
    fn infinity() -> Self {
        Self {
            x: Field::ZERO,
            y: Field::ZERO,
            inf: true,
        }
    }

    fn new(x: Field, y: Field) -> Self {
        let p = Self { x, y, inf: false };
        debug_assert!(p.is_on_curve());
        p
    }

    #[cfg(test)]
    fn negate(&self) -> Self {
        if self.inf {
            return self.clone();
        }
        Self::new(self.x, self.x.add(&self.y))
    }

    fn is_on_curve(&self) -> bool {
        if self.inf {
            return true;
        }

        // 文档验证结果对应的是 y^2 + xy = x^3 + 161
        let lhs = self.y.square().add(&self.x.mul(&self.y));
        let rhs = self.x.square().mul(&self.x).add(&curve_b());
        lhs == rhs
    }

    fn double(&self) -> Self {
        if self.inf || self.x.is_zero() {
            return Self::infinity();
        }

        let m = self.y.div(&self.x).add(&self.x);
        let new_x = m.square().add(&m);
        let new_y = m.add_one().mul(&new_x).add(&self.x.square());
        Self::new(new_x, new_y)
    }

    fn add(&self, other: &Self) -> Self {
        // 二元域曲线上的加法公式
        if self.inf {
            return other.clone();
        }
        if other.inf {
            return self.clone();
        }
        if self.x == other.x {
            return if self.y == other.y {
                self.double()
            } else {
                Self::infinity()
            };
        }

        let m = self.y.add(&other.y).div(&self.x.add(&other.x));
        let new_x = m.square().add(&m).add(&self.x).add(&other.x);
        let new_y = m.mul(&self.x.add(&new_x)).add(&new_x).add(&self.y);
        Self::new(new_x, new_y)
    }

    fn mul(&self, n: &BigUint) -> Self {
        // 低位到高位的 double-and-add 标量乘
        let mut acc = Point::infinity();
        let mut base = self.clone();
        let mut k = n.clone();

        while !k.is_zero() {
            if (&k & BigUint::one()) == BigUint::one() {
                acc = acc.add(&base);
            }
            base = base.double();
            k >>= 1u32;
        }

        acc
    }

    // WinRAR 把压缩公钥重新编码成 64 位十六进制字符串。
    fn sm2_string(&self) -> String {
        let z = if self.x.is_zero() {
            self.y.lsb()
        } else {
            self.y.div(&self.x).lsb()
        };

        let mut packed = self.x.to_biguint() << 1usize;
        if z == 1 {
            packed += BigUint::one();
        }
        hex_pad(&packed, 64)
    }
}

fn curve_b() -> Field {
    let mut v = [0u16; FIELD_DEGREE];
    v[0] = 161;
    Field::from_limbs(v)
}

fn base_point() -> &'static Point {
    static BASE: OnceLock<Point> = OnceLock::new();
    BASE.get_or_init(|| Point::new(Field::from_limbs(G_X), Field::from_limbs(G_Y)))
}

#[cfg(test)]
fn master_public_key() -> &'static Point {
    static PK: OnceLock<Point> = OnceLock::new();
    PK.get_or_init(|| {
        Point::new(
            Field::from_limbs(PUBLIC_KEY_X),
            Field::from_limbs(PUBLIC_KEY_Y),
        )
    })
}

fn order() -> &'static BigUint {
    static N: OnceLock<BigUint> = OnceLock::new();
    N.get_or_init(|| BigUint::parse_bytes(ORDER_HEX.as_bytes(), 16).unwrap())
}

fn master_private_key() -> &'static BigUint {
    static K: OnceLock<BigUint> = OnceLock::new();
    K.get_or_init(|| BigUint::parse_bytes(MASTER_PRIVATE_KEY_HEX.as_bytes(), 16).unwrap())
}

fn sha1_words(data: &[u8]) -> [u32; 5] {
    // 按大端切成 5 个 32-bit 状态字 与文档表述一致
    let digest = Sha1::digest(data);
    let mut words = [0u32; 5];
    for (i, chunk) in digest.chunks_exact(4).enumerate() {
        words[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    words
}

fn biguint_from_u16_le(words: &[u16]) -> BigUint {
    let mut bytes = Vec::with_capacity(words.len() * 2);
    for &w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    BigUint::from_bytes_le(&bytes)
}

fn generate_private_key(seed: &[u8]) -> BigUint {
    // 按文档中的 6 个 32-bit 生成器与 15 次 SHA1 派生私钥
    let mut g = [0u32; 6];

    if seed.is_empty() {
        g[1] = 0xeb3eb781;
        g[2] = 0x50265329;
        g[3] = 0xdc5ef4a3;
        g[4] = 0x6847b9d5;
        g[5] = 0xcde43b4c;
    } else {
        let words = sha1_words(seed);
        g[1..].copy_from_slice(&words);
    }

    let mut out = [0u16; 15];
    for i in 0..15 {
        g[0] = (i + 1) as u32;
        let mut bytes = Vec::with_capacity(24);
        for word in g {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        out[i] = sha1_words(&bytes)[0] as u16;
    }

    biguint_from_u16_le(&out)
}

fn generate_hash_integer(data: &[u8]) -> BigUint {
    // WinRAR 没直接使用标准 SHA1 序列化 而是拼成 240 bit 整数
    let words = sha1_words(data);
    let mut out = [0u16; 15];

    for (i, word) in words.iter().enumerate() {
        out[i * 2] = (*word & 0xffff) as u16;
        out[i * 2 + 1] = (*word >> 16) as u16;
    }

    out[10] = 0x8d43;
    out[11] = 0x0ffd;
    out[12] = 0x3c7c;
    out[13] = 0xb4e3;
    out[14] = 0x1bd1;

    biguint_from_u16_le(&out)
}

fn mod_sub(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
    if a >= b {
        (a - b) % m
    } else {
        let d = (b - a) % m;
        if d.is_zero() { BigUint::zero() } else { m - d }
    }
}

fn random_scalar(rng: &mut impl Rng) -> BigUint {
    // 这里直接生成 240 bit 随机数 与原始实现的 rand() 思路一致
    loop {
        let mut limbs = [0u16; 15];
        for limb in &mut limbs {
            *limb = rng.random::<u32>() as u16;
        }
        let k = biguint_from_u16_le(&limbs);
        if !k.is_zero() {
            return k;
        }
    }
}

fn sign(data: &[u8], rng: &mut impl Rng) -> (BigUint, BigUint) {
    // 使用文档中给出的 WinRAR 变体签名流程
    let h = generate_hash_integer(data);

    loop {
        let rnd = random_scalar(rng);
        let rp = base_point().mul(&rnd);
        let r = (rp.x.to_biguint() + &h) % order();
        if r.is_zero() || (&r + &rnd) == *order() {
            continue;
        }

        let kr = (master_private_key() * &r) % order();
        let s = mod_sub(&rnd, &kr, order());
        if !s.is_zero() {
            return (r, s);
        }
    }
}

fn hex_pad(n: &BigUint, width: usize) -> String {
    let s = n.to_str_radix(16);
    if s.len() >= width {
        s
    } else {
        format!("{:0>width$}", s, width = width)
    }
}

fn public_key_sm2_bytes(seed: &[u8]) -> String {
    // 先由输入派生私钥 再做标量乘 最后转成 WinRAR 需要的压缩格式字符串
    let private_key = generate_private_key(seed);
    base_point().mul(&private_key).sm2_string()
}

pub struct RegisterInfo {
    pub username: String,
    pub license_type: String,
    pub uid: String,
    pub items: [String; 4],
    pub checksum: u32,
    pub hex_data: String,
}

fn crc32_checksum(parts: &[&[u8]]) -> u32 {
    let mut crc = Crc32::new();
    for part in parts {
        crc.update(part);
    }
    !crc.finalize()
}

fn sign_item(data: &[u8], rng: &mut impl Rng) -> String {
    // 文档要求 r 和 s 都必须限制在 60 个十六进制字符内
    loop {
        let (r, s) = sign(data, rng);
        let rs = hex_pad(&r, 60);
        let ss = hex_pad(&s, 60);
        if rs.len() == 60 && ss.len() == 60 {
            return format!("60{}{}", ss, rs);
        }
    }
}

pub fn generate_register_info(
    username: &str,
    license_type: &str,
    rng: &mut impl Rng,
) -> RegisterInfo {
    // 当前直接使用 UTF-8 字节 若要完全复刻 WinRAR ANSI 可只替换这里的编码层
    let user_bytes = username.as_bytes();
    let license_bytes = license_type.as_bytes();

    // Data0 ~ Data3 与 UID 完全按文档拼装
    let temp = public_key_sm2_bytes(user_bytes);
    let data3 = format!("60{}", &temp[..48]);
    let data0 = public_key_sm2_bytes(data3.as_bytes());
    let uid = format!("{}{}", &temp[48..64], &data0[..4]);
    let data1 = sign_item(license_bytes, rng);

    let mut temp2 = Vec::with_capacity(user_bytes.len() + data0.len());
    temp2.extend_from_slice(user_bytes);
    temp2.extend_from_slice(data0.as_bytes());
    let data2 = sign_item(&temp2, rng);

    let checksum = crc32_checksum(&[
        license_bytes,
        user_bytes,
        data0.as_bytes(),
        data1.as_bytes(),
        data2.as_bytes(),
        data3.as_bytes(),
    ]);

    let hex_data = format!(
        "{}{}{}{}{}{}{}{}{:010}",
        data0.len(),
        data1.len(),
        data2.len(),
        data3.len(),
        data0,
        data1,
        data2,
        data3,
        checksum
    );

    RegisterInfo {
        username: username.to_string(),
        license_type: license_type.to_string(),
        uid,
        items: [data0, data1, data2, data3],
        checksum,
        hex_data,
    }
}

pub fn format_license(info: &RegisterInfo) -> String {
    // 输出为 WinRAR 可识别的授权文本格式
    let mut out = String::new();
    out.push_str("RAR registration data\r\n");
    out.push_str(&info.username);
    out.push_str("\r\n");
    out.push_str(&info.license_type);
    out.push_str("\r\nUID=");
    out.push_str(&info.uid);
    out.push_str("\r\n");

    for chunk in info.hex_data.as_bytes().chunks(54) {
        out.push_str(std::str::from_utf8(chunk).unwrap());
        out.push_str("\r\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_points_on_curve() {
        assert!(base_point().is_on_curve());
        assert!(master_public_key().is_on_curve());
    }

    #[test]
    fn empty_seed_private_key_matches_doc() {
        assert_eq!(generate_private_key(&[]), *master_private_key());
    }

    #[test]
    fn empty_seed_public_key_matches_doc() {
        let pk = base_point().mul(master_private_key());
        assert_eq!(pk, *master_public_key());
    }

    #[test]
    fn point_negation_roundtrip() {
        let g = base_point();
        assert_eq!(g.add(&g.negate()), Point::infinity());
    }
}
