//! Handy build scripts.

use std::{env, fs::File, io::Write, path::Path, process::Command};

fn main() {
    // Compute the hash of the current Git commit.
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("unable to get git version");
    let git_hash = String::from_utf8(output.stdout).expect("could not parse git version");
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut bishop_table_file =
        File::create(Path::join(out_dir.as_ref(), "bishop_magic_table.rs")).unwrap();
    let mut rook_table_file =
        File::create(Path::join(out_dir.as_ref(), "rook_magic_table.rs")).unwrap();

    let bishop_table = construct_magic_table(&BISHOP_BITS, &SAVED_BISHOP_MAGICS, false);
    write!(
        bishop_table_file,
        "
            #[allow(clippy::unreadable_literal, clippy::large_stack_arrays, clippy::large_const_arrays)]
            const BISHOP_ATTACKS_TABLE: [Bitboard; {}] = unsafe {{ 
                std::mem::transmute::<[u64; {0}], [Bitboard; {0}]>({:?}) 
            }};
        ",
        bishop_table.len(),
        bishop_table
    )
    .unwrap();

    let rook_table = construct_magic_table(&ROOK_BITS, &SAVED_ROOK_MAGICS, true);
    write!(
        rook_table_file,
        "
            #[allow(clippy::unreadable_literal, clippy::large_stack_arrays, clippy::large_const_arrays)]
            const ROOK_ATTACKS_TABLE: [Bitboard; {}] = unsafe {{ 
                std::mem::transmute::<[u64; {0}], [Bitboard; {0}]>({:?}) 
            }};
        ",
        rook_table.len(),
        rook_table
    )
    .unwrap();
}

/* BELOW: cursed implemention of magic table generation with no type safety at all */

/// Type alias for bitboards.
type Bitboard = u64;
/// A square, from 0 to 63 for A1 to H8.
type Square = u8;
/// A direction on a board that can be added to a Square.
type Direction = i8;

/// A saved list of magics for rooks created using the generator.
///
/// Some magics for sizes below the required bitshift amount were taken from the Chess Programming
/// Wiki.
const SAVED_ROOK_MAGICS: [u64; 64] = [
    0x4080_0020_4000_1480, // a1
    0x0040_0010_0140_2000, // b1
    0x0300_2000_1810_4100, // c1
    0x2100_0409_0110_0120, // d1
    0x8a00_0600_0408_2070, // e1
    0x0080_0144_0002_0080, // f1
    0x1100_2500_208a_0004, // g1
    0x0900_0042_2201_8100, // h1
    0x0208_8002_28c0_0081, // a2
    0x2280_4010_0340_2000, // b2
    0x0008_8010_0020_0184, // c2
    0x0001_0020_1000_0900, // d2
    0x0182_0006_0010_6008, // e2
    0x2058_8004_0080_0200, // f2
    0x0004_8002_0080_0900, // g2
    0x052d_0012_0040_a100, // h2
    0x0540_0880_0080_24c1, // a3
    0x2000_8480_4002_2000, // b3
    0x0400_4100_1100_6000, // c3
    0x0040_a100_3001_0108, // d3
    0x1204_8080_0800_0402, // e3
    0x0802_8080_0400_2201, // f3
    0x1002_8080_5200_0500, // g3
    0x0004_0a00_2112_4184, // h3
    0x0640_0128_8008_8040, // a4
    0x8410_4000_8020_008a, // b4
    0x0400_2008_8010_0080, // c4
    0x2001_0121_0009_1004, // d4
    0x1200_0d01_0008_0010, // e4
    0x6004_0004_0120_1008, // f4
    0x7500_aa04_0008_4110, // g4
    0x0100_0052_0004_0981, // h4
    0x0040_8040_0280_0020, // a5
    0x0470_0020_0640_0240, // b5
    0x0001_2000_8080_1000, // c5
    0x0000_0812_0200_2040, // d5
    0x00c0_8044_0080_0800, // e5
    0x9000_800a_0080_0400, // f5
    0x0001_0004_0100_0600, // g5
    0x0042_1088_ca00_2401, // h5
    0x0000_c000_228d_8000, // a6
    0x6410_0420_1440_4001, // b6
    0x1002_0040_8226_0014, // c6
    0x206a_0088_11c2_0021, // d6
    0x0002_0018_1022_0024, // e6
    0x2001_0200_0400_8080, // f6
    0x1000_0801_100c_001a, // g6
    0x0048_0082_5402_0011, // h6
    0x48FF_FE99_FECF_AA00, // a7, found by Grant Osborne
    0x48FF_FE99_FECF_AA00, // b7, found by Grant Osborne
    0x497F_FFAD_FF9C_2E00, // c7, found by Grant Osborne
    0x613F_FFDD_FFCE_9200, // d7, found by Grant Osborne
    0xffff_ffe9_ffe7_ce00, // e7, found by Volker Annuss
    0xffff_fff5_fff3_e600, // f7, found by Volker Annuss
    0x0003_ff95_e5e6_a4c0, // g7, found by Niklas Fiekas
    0x510F_FFF5_F63C_96A0, // h7, found by Grant Osborne
    0xEBFF_FFB9_FF9F_C526, // a8, found by Grant Osborne
    0x61FF_FEDD_FEED_AEAE, // b8, found by Grant Osborne
    0x53BF_FFED_FFDE_B1A2, // c8, found by Grant Osborne
    0x127F_FFB9_FFDF_B5F6, // d8, found by Grant Osborne
    0x411F_FFDD_FFDB_F4D6, // e8, found by Grant Osborne
    0x0822_0024_0810_4502, // f8
    0x0003_ffef_27ee_be74, // g8, found by Peter Österlund
    0x7645_FFFE_CBFE_A79E, // h8, found by Grant Osborne
];

/// A saved list of magics for bishops created using the generator.
///
/// Some magics for sizes below the required bitshift amount were taken from the Chess Programming
/// Wiki.
const SAVED_BISHOP_MAGICS: [u64; 64] = [
    0xffed_f9fd_7cfc_ffff, // a1, found by Gerd Isenberg
    0xfc09_6285_4a77_f576, // b1, found by Gerd Isenberg
    0x0012_2808_c102_a004, // c1
    0x2851_2400_8240_0440, // d1
    0x0011_1040_1100_0202, // e1
    0x0008_2208_2000_0010, // f1
    0xfc0a_66c6_4a7e_f576, // g1, found by Gerd Isenberg
    0x7ffd_fdfc_bd79_ffff, // h1, found by Gerd Isenberg
    0xfc08_46a6_4a34_fff6, // a2, found by Gerd Isenberg
    0xfc08_7a87_4a3c_f7f6, // b2, found by Gerd Isenberg
    0x0009_8802_0420_a000, // c2
    0x8000_4404_0080_8200, // d2
    0x208c_8450_c001_3407, // e2
    0x1980_1105_2010_8030, // f2
    0xfc08_64ae_59b4_ff76, // g2, found by Gerd Isenberg
    0x3c08_60af_4b35_ff76, // h2, found by Gerd Isenberg
    0x73C0_1AF5_6CF4_CFFB, // a3, found by Richard Pijl
    0x41A0_1CFA_D64A_AFFC, // b3, found by Richard Pijl
    0x0604_0002_04a2_0202, // c3
    0x0002_8208_0602_4000, // d3
    0x008a_0024_2201_0201, // e3
    0x2082_0040_8801_0802, // f3
    0x7c0c_028f_5b34_ff76, // g3, found by Gerd Isenberg
    0xfc0a_028e_5ab4_df76, // h3, found by Gerd Isenberg
    0x0810_0420_d104_1080, // a4
    0x0904_5100_0210_0100, // b4
    0x0202_2808_0406_4403, // c4
    0x004c_0040_0c03_0082, // d4
    0x0602_0010_0200_5011, // e4
    0x7209_0200_c108_9000, // f4
    0x4211_4104_2400_8805, // g4
    0x0002_8484_2126_0804, // h4
    0xc001_0412_1121_2004, // a5
    0x0208_0188_0004_4800, // b5
    0x0080_2064_1058_0800, // c5
    0x0000_2011_0008_0084, // d5
    0x0208_0034_0009_4100, // e5
    0x2190_4102_0000_4058, // f5
    0x0188_8214_0180_8080, // g5
    0x2006_0a02_0000_c4c0, // h5
    0xDCEF_D9B5_4BFC_C09F, // a6, found by Richard Pijl
    0xF95F_FA76_5AFD_602B, // b6, found by Richard Pijl
    0x200a_1041_1000_2040, // c6
    0x0800_000c_0831_0c00, // d6
    0x0218_0401_0a01_0400, // e6
    0x1092_2004_0022_4100, // f6
    0x43ff_9a5c_f4ca_0c01, // g6, found by Gerd Isenberg
    0x4BFF_CD8E_7C58_7601, // h6, found by Richard Pijl
    0xfc0f_f286_5334_f576, // a7, found by Gerd Isenberg
    0xfc0b_f6ce_5924_f576, // b7, found by Gerd Isenberg
    0x8052_2060_8c30_0001, // c7
    0x2084_1050_4202_0400, // d7
    0xe018_8010_2206_0220, // e7
    0x0001_1220_4901_0200, // f7
    0xc3ff_b7dc_36ca_8c89, // g7, found by Gerd Isenberg
    0xc3ff_8a54_f4ca_2c89, // h7, found by Gerd Isenberg
    0xffff_fcfc_fd79_edff, // a8, found by Gerd Isenberg
    0xfc08_63fc_cb14_7576, // b8, found by Gerd Isenberg
    0x40a0_0400_6213_3000, // c8
    0x0142_0280_0084_0400, // d8
    0x0009_0900_1006_1200, // e8
    0x0800_8445_2810_0308, // f8
    0xfc08_7e8e_4bb2_f736, // g8, found by Gerd Isenberg
    0x43ff_9e4e_f4ca_2c89, // h8, found by Gerd Isenberg
];

/// The number of bits used to express the magic lookups for rooks at each square.
const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, // rank 1
    11, 10, 10, 10, 10, 10, 10, 11, // 2
    11, 10, 10, 10, 10, 10, 10, 11, // 3
    11, 10, 10, 10, 10, 10, 10, 11, // 4
    11, 10, 10, 10, 10, 10, 10, 11, // 5
    11, 10, 10, 10, 10, 10, 10, 11, // 6
    10, 9, 9, 9, 9, 9, 9, 10, // 7
    11, 10, 10, 10, 10, 11, 10, 11, // 8
];

/// The number of bits used to express the magic lookups for bishops at each square.
const BISHOP_BITS: [u8; 64] = [
    5, 4, 5, 5, 5, 5, 4, 5, // rank 1
    4, 4, 5, 5, 5, 5, 4, 4, // 2
    4, 4, 7, 7, 7, 7, 4, 4, // 3
    5, 5, 7, 9, 9, 7, 5, 5, // 4
    5, 5, 7, 9, 9, 7, 5, 5, // 5
    4, 4, 7, 7, 7, 7, 4, 4, // 6
    4, 4, 5, 5, 5, 5, 4, 4, // 7
    5, 4, 5, 5, 5, 5, 4, 5, // 8
];

const ROOK_DIRECTIONS: [Direction; 4] = [1, -1, 8, -8];
const BISHOP_DIRECTIONS: [Direction; 4] = [9, -9, 7, -7];

fn construct_magic_table(bits: &[u8; 64], magics: &[u64; 64], is_rook: bool) -> Vec<Bitboard> {
    let mut table = vec![0; bits.iter().map(|x| 1 << x).sum()];

    let mut table_offset = 0;
    for (i, (&table_bits, &magic)) in bits.iter().zip(magics.iter()).enumerate() {
        let sq = i as u8;
        let mask = if is_rook {
            get_rook_mask(sq)
        } else {
            get_bishop_mask(sq)
        };
        for attack_id in 0..(1 << mask.count_ones()) {
            let occupancy = pdep(attack_id, mask);
            let attack = directional_attacks(
                sq,
                if is_rook {
                    &ROOK_DIRECTIONS
                } else {
                    &BISHOP_DIRECTIONS
                },
                occupancy,
            );
            let key = compute_magic_key(occupancy, magic, 64 - table_bits);
            assert!(table[key + table_offset] == 0 || table[key + table_offset] == attack);
            table[key + table_offset] = attack;
        }

        table_offset += 1 << table_bits;
    }

    table
}

/// Create the mask for the relevant bits in magic of a rook.
/// `sq` is the identifying the square that we want to generate the mask for.
fn get_rook_mask(sq: Square) -> Bitboard {
    // sequence of 1s down the same row as the piece to move, except on the ends
    let row_mask = 0x7E << (sq & !0x7);
    // sequence of 1s down the same col as the piece to move, except on the ends
    let col_mask = 0x0001_0101_0101_0100 << (sq & 0x7);
    // note: pieces at the end of the travel don't matter, which is why the masks aren't uniform

    // in the col mask or row mask, but not the piece to move
    (row_mask | col_mask) & !(1 << sq as u64)
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
/// Create the mask for the relevant bits in magic of a bishop.
/// `sq` is the square that a bishop would be on to receiver this mask.
fn get_bishop_mask(sq: Square) -> Bitboard {
    const MAIN_DIAG: Bitboard = 0x8040_2010_0804_0201;
    const ANTI_DIAG: Bitboard = 0x0102_0408_1020_4080;
    const RING_MASK: Bitboard = 0xFF81_8181_8181_81FF;

    // thank u chessprogramming wiki for this code
    let i = sq as i32;
    let main_diag = 8 * (i & 7) - (i & 56);
    let main_left_shift = (-main_diag & (main_diag >> 31)) as u8;
    let main_right_shift = (main_diag & (-main_diag >> 31)) as u8;
    let main_diag_mask = (MAIN_DIAG >> main_right_shift) << main_left_shift;

    let anti_diag = 56 - 8 * (i & 7) - (i & 56);
    let anti_left_shift = (-anti_diag & (anti_diag >> 31)) as u8;
    let anti_right_shift = (anti_diag & (-anti_diag >> 31)) as u8;
    let anti_diag_mask = (ANTI_DIAG >> anti_right_shift) << anti_left_shift;

    (main_diag_mask ^ anti_diag_mask) & !RING_MASK
}

#[allow(clippy::cast_possible_truncation)]
/// Use magic hashing to get the index to look up attacks in a bitboard.
fn compute_magic_key(occupancy: Bitboard, magic: u64, shift: u8) -> usize {
    (occupancy.wrapping_mul(magic) >> shift) as usize
}

/// Manual implementation of the parallel bits deposit instruction.
fn pdep(index: usize, mut mask: Bitboard) -> Bitboard {
    let mut result = 0u64;
    // go from right to left in the bits of num_points,
    // and add an occupancy if something is there
    for i in 0..mask.count_ones() {
        // make a bitboard which only occupies the rightmost square
        let occupier = mask & !(mask - 1);
        // remove the occupier from the mask
        mask ^= occupier;
        if (index & (1 << i)) != 0 {
            // the bit corresponding to the occupier is nonzero
            result |= occupier;
        }
    }

    result
}

fn directional_attacks(sq: Square, dirs: &[Direction], occupancy: Bitboard) -> Bitboard {
    let mut result = 0;
    for &dir in dirs {
        let mut current_sq = sq;
        for _ in 0..8 {
            let next_sq_i16: i16 = current_sq as i16 + dir as i16;
            if !(0..64).contains(&next_sq_i16) {
                break;
            }
            let next_sq = next_sq_i16 as u8;
            if (next_sq & 0x7).abs_diff(current_sq & 0x7) > 1
                || (next_sq / 8).abs_diff(current_sq / 8) > 1
            {
                break;
            }
            result |= 1 << next_sq;
            if (occupancy & 1 << next_sq) != 0 {
                break;
            }
            current_sq = next_sq;
        }
    }

    result
}
